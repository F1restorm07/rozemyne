use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;
use crossbeam::channel::{self, Receiver};
use highway::HighwayHash;
use notify_debouncer_full::notify::event::CreateKind;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer, notify};
use notify_debouncer_full::notify::RecommendedWatcher;

use crate::media::MediaType;
use crate::parser::keyword::{KeywordFlag, Language};
use crate::parser::{Segment, SegmentKind};
use crate::renamer::{self, Filterable};

#[derive(Debug, Component, Clone, mlua::UserData, mlua::FromLua)]
pub struct MediaFile {
    hash: [u64; 4],
    path: PathBuf,
    // the media folder from where the file originates
    folder_id: String,
}

impl MediaFile {
    pub fn new(path: impl AsRef<Path>, hash_buf: usize, folder_id: String) -> Self {
        let mut reader = File::open(&path).unwrap();
        let mut buf = Vec::with_capacity(hash_buf);
        let mut hasher = highway::HighwayHasher::default();

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => hasher.append(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => panic!("hit error while hashing file: {e:?}"),
            }
        }

        Self {
            hash: hasher.finalize256(),
            path: path.as_ref().to_path_buf(),
            folder_id,
        }
    }

    pub fn hash(&self) -> &[u8; 32] { unsafe { std::mem::transmute(&self.hash) } }
    pub fn path(&self) -> &Path { &self.path }
    pub fn path_mut(&mut self) -> &mut PathBuf { &mut self.path }
    pub fn folder_id(&self) -> &str { &self.folder_id }
}

// marker component for unprocessed files
#[derive(Component)]
pub struct UnprocessedFile;
#[derive(Component)]
pub struct NeedsMetadata;
#[derive(Component)]
pub struct WaitingForMetadata;
// the intial metadata request has failed, and this is the nth retry
#[derive(Component)]
pub struct MetadataRetry(pub u8);

// @todo: implement spaces for *all* the title parts
// ^^ can i implement this as a hashmap?
#[derive(Debug, Component)]
pub struct ParsedFilename {
    // this can (and will) be updated in response to metadata query fails
    pub title: String,
    pub episode_title: Option<String>,
    pub release_group: Option<String>,

    // @todo: support ranges (i.e. v01-18)
    pub season: Option<u32>,
    // @todo: add type to capture ranges + sets here (alongside single episodes)
    pub episode: Option<u32>,
    pub volume: Option<u32>,
    pub chapter: Option<u32>,

    pub year: Option<u32>,

    pub service: Option<String>,
    pub rip_source: Option<String>,
    // @todo: add language enum(set) here (embed within the keyword)
    pub languages: Vec<Language>,

    pub resolution: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<String>,
    pub video_codec: Option<String>,

    pub keyword_flags: enumset::EnumSet<KeywordFlag>,

    pub file_extension: Option<String>,
    pub version: Option<u32>,
}

impl ParsedFilename {
    pub fn new(title_parts: &[Segment]) -> Self {
        println!("{title_parts:?}");
        if title_parts.iter().any(|p| matches!(p.kind, SegmentKind::Title)) {
            let title = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Title))
                .map(|p| p.text.to_string()).unwrap();
            let episode_title = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::EpisodeTitle))
                .map(|p| p.text.to_string());
            let release_group = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Group))
                .map(|p| p.text.to_string());

            // the season may be parsed as an episode if it's a freestanding number
            // at the end of the title
            let season = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Season))
                .map(|p| p.text.parse().unwrap());
            let episode = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Episode))
                .map(|p| p.text.parse().unwrap());
            // @todo: support volume ranges (they produce multiple `Volume` title parts)
            let volume = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Volume) && p.text.parse::<usize>().is_ok())
                .map(|p| p.text.parse().unwrap());
            let chapter = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Chapter) && p.text.parse::<usize>().is_ok())
                .map(|p| p.text.parse().unwrap());

            let year = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Year))
                .map(|p| p.text.parse().unwrap());

            let service = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Service))
                .map(|p| p.text.to_string());
            let rip_source = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Source))
                .map(|p| p.text.to_string());

            let languages = title_parts.iter()
                .filter(|p| matches!(p.kind, SegmentKind::Language(_)))
                .map(|p| {
                    let SegmentKind::Language(l) = p.kind else { panic!("uh-oh") };
                    l
                }).collect();

            let resolution = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Resolution))
                .map(|p| p.text.to_string());
            let audio_codec = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::AudioCodec))
                .map(|p| p.text.to_string());
            let audio_channels = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::AudioChannels))
                .map(|p| p.text.to_string());
            let video_codec = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::VideoCodec))
                .map(|p| p.text.to_string());

            let keyword_flags = title_parts.iter()
                .filter_map(|p| KeywordFlag::from_segment(p.kind))
                .collect::<enumset::EnumSet<_>>();

            let file_extension = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::FileExtension))
                .map(|p| p.text.to_string());
            let version = title_parts.iter()
                .find(|p| matches!(p.kind, SegmentKind::Version))
                .map(|p| p.text.parse().unwrap());

            Self {
                title,
                episode_title,
                release_group,
                season,
                episode,
                volume,
                chapter,
                year,
                service,
                rip_source,
                languages,
                resolution,
                audio_codec,
                audio_channels,
                video_codec,
                keyword_flags,
                file_extension,
                version,
            }

        } else {
            panic!("no title was found for the given file");
        }
    }
}

impl Filterable for ParsedFilename {
    fn get_context(&self) -> crate::renamer::Context {
        let mut context: renamer::Context = [
            ("file.title".parse().unwrap(), self.title.as_str().into())
        ].into();

        if let Some(season) = self.season {
            context.insert("file.season", season as u64);
        }
        if let Some(episode) = self.episode {
            context.insert("file.episode", episode as u64);
        }
        if let Some(volume) = self.volume {
            context.insert("file.volume", volume as u64);
        }

        context
    }
}

// a clone of bevy_asset's `FileWatcher`
pub struct FileWatcher {
    debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    recv: Receiver<FileEvent>,
}

pub struct FileEvent {
    path: PathBuf,
}

impl FileEvent {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
        }
    }

    pub fn path(&self) -> &Path { &self.path }
}

impl FileWatcher {
    pub fn new(path: impl AsRef<Path>, debounce_time: Duration) -> Self {
        let (tx, rx) = channel::bounded(8);
        let mut this = Self {
            debouncer: new_debouncer(debounce_time, None, move |res: DebounceEventResult| {
                match res {
                    Ok(ev) => {
                        for ev in ev.iter() {
                            match ev.kind {
                                // @todo: check if modified path is a file
                                notify::EventKind::Create(CreateKind::File)
                                    | notify::EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::To))
                                => {
                                    tx.send(FileEvent::new(ev.paths[0].clone())).unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(errs) => errs.iter().for_each(|e|
                        eprintln!("encountered a file system error: {e:?}")
                    ),
                }
            }).unwrap(),
            recv: rx,
        };
        this.debouncer.watch(path, notify::RecursiveMode::Recursive).unwrap();
        this
    }

    pub fn try_recv(&self) -> Result<FileEvent, channel::TryRecvError> {
        self.recv.try_recv()
    }
    pub fn try_iter(&self) -> channel::TryIter<'_, FileEvent> {
        self.recv.try_iter()
    }
}

#[derive(Resource)]
pub struct FileServer {
    watchers: HashMap<FileWatcherId, FileWatcher>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FileWatcherId {
    id: String,
    media_type: MediaType,
}

impl FileWatcherId {
     pub fn new(id: impl Into<String>, media_type: MediaType) -> Self {
         Self { id: id.into(), media_type }
     }

     pub fn media_type(&self) -> MediaType { self.media_type }
     pub fn id(&self) -> &str { &self.id }
}

impl FileServer {
    pub fn new(watchers: HashMap<FileWatcherId, FileWatcher>) -> Self {
        Self { watchers }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FileWatcherId, &FileWatcher)> {
        self.watchers.iter()
    }
}
