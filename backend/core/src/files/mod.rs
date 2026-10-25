// file processing pipeline
// 1. check for files in source (import) folder
//  a. if a file watcher is active, notify on file import
// 2. import file into rozemyne
//  a. hash file, check if file is duplicate based on hash and title using database
//  b. parse filename
//  c. pull metadata in for file using data sources, clean and store in database
//  d. store file data in database (after move)
// 3. if a destination folder exists, move file
//  a. if a renamer is active, rename file before moving
//  b. move file into correct media type's folder directory
//  - the destination folder can be the same as the source folder
//      - in this instance, all the processing will be performed without moving the file

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use backend_types::files::{FileEvent, FileServer, FileWatcher, FileWatcherId, MediaFile, MetadataRetry, ParsedFilename, UnprocessedFile, WaitingForMetadata};
use backend_types::media::data_source::{DataQueryType, DataSourceRequest, DataSourceResponse, Media, MediaInternalRequest, MediaRequest, MediaRequestId};
use backend_types::media::query::MediaTitleQuery;
use backend_types::media::{ItemUuid, MediaId, MediaTitles, MediaType, TitleTrait};
use backend_types::parser::SegmentKind;
use backend_types::parser::token::{Lexer, TokenIter};
use backend_types::renamer::{Context, Filterable};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{Or, With, Without};
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Commands, Query, Res};
use bevy_ecs::world::EntityRef;

use crate::configuration::Settings;

use self::renamer::PatternEngine;

pub mod parser;
pub mod renamer;

// @todo: only load this on the `Server` command
pub struct FileLoadingPlugin;

impl bevy_app::Plugin for FileLoadingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let config = app.world().get_resource_ref::<Settings>().unwrap();
        let watchers = config.media_folders().iter().map(|f| {
            (
                FileWatcherId::new(f.id(), f.media_type()),
                FileWatcher::new(f.import_path(), Duration::from_millis(500)))
        }).collect::<HashMap<_, _>>();
        app
            .insert_resource(FileServer::new(watchers))
            .add_systems(bevy_app::PreUpdate, (process_watcher_events, query_file_metadata).chain())
            .add_systems(bevy_app::PostUpdate, (check_file_metadata, rename_and_move_files).chain());
    }
}

fn process_watcher_events(
    server: Res<FileServer>,
    mut cmds: Commands,
) {
    for (id, watcher) in server.iter() {
        cmds.spawn_batch(watcher.try_iter().map(|evt| {
            (UnprocessedFile, ItemUuid(uuid::Uuid::now_v7()), MediaFile::new(evt.path(), 16, id.id().to_string()), id.media_type())
        }).collect::<Vec<_>>());
    }
}

fn query_file_metadata(
    files: Query<(Entity, &ItemUuid, &MediaFile, &MediaType), (With<UnprocessedFile>, Without<MediaId>)>,
    mut cmds: Commands,
) {
    // @todo: group media requests by path (otherwise multiple requests with the same parameters will be sent)
    for (ent, ItemUuid(id), file, media_type) in files {
        // let title_parts = parser::parse(TokenIter::new(file.path().file_name().unwrap().to_str().unwrap()).collect());
        let title_parts = parser::parse(Lexer::new(file.path().file_name().unwrap().to_str().unwrap()).into_tokens());
        let parsed = ParsedFilename::new(&title_parts);
        let req = MediaInternalRequest::new(DataQueryType::Search, *media_type);
        let req_id = req.id();

        // @todo: insert more query parameters here
        cmds.spawn((
            req,
            MediaTitleQuery(parsed.title.clone())
        ));
        // we store the data request id here for future reference
        cmds.entity(ent).insert((WaitingForMetadata, MediaRequestId(req_id), parsed));
    }
}

// checks for whether the metadata query successfully completed
fn check_file_metadata(
    // @todo: swap out the media request type with another newtype wrapper
    mut files: Query<(Entity, &MediaRequestId, &MediaType, &mut ParsedFilename, Option<&mut MetadataRetry>), With<WaitingForMetadata>>,
    // @todo: reformat this to `MediaInternalResponse`
    ecs_responses: Query<(&MediaRequestId, &MediaId)>,
    media: Query<
        &ItemUuid,
        // we need to know if the search returned anything
        With<Media>,
    >,
    mut cmds: Commands
) {
    for (ent, req_id, media_type, mut parsed, retry) in files.iter_mut() {
        if let Some(resp) = ecs_responses.iter().find(|(id, _)| id.0 == req_id.0) {
            // hit media within ECS system
            cmds.entity(ent).remove::<WaitingForMetadata>();
            cmds.entity(ent).insert(resp.1.clone());
        } else if media.iter().any(|id| id.0 == req_id.0) {
            // we found the media
            cmds.entity(ent).remove::<WaitingForMetadata>();
            // associate the file with the media
            cmds.entity(ent).insert(MediaId(req_id.0));
        } else {
            // no media found, retry
            let retry_n = match retry {
                None => {
                    cmds.entity(ent).insert(MetadataRetry(1));
                    1
                }
                Some(mut r) => {
                    r.0 += 1;
                    r.0
                }
            };
            let Some(new_title) = retry_title(&parsed.title, retry_n) else {
                // we have fallen through every retry mechanism, or the title is empty
                panic!("no media found for the given title {:?}", parsed.title);
            };
            let new_title = new_title.to_string();
            parsed.title = new_title.clone();
            cmds.spawn((
                MediaRequest::with_id(req_id.0, *media_type),
                MediaTitleQuery(new_title)
            ));
        }
    }
}

// @todo: check if the file's path is already occupied (or another file with the same metadata exists)
fn rename_and_move_files(
    settings: Res<Settings>,
    files: Query<(Entity, &MediaId, &mut MediaFile, &MediaType, &ParsedFilename), With<UnprocessedFile>>,
    // @todo: figure out what to pull in here (and what goes in the pattern engine?)
    // `EntityRef` is here since every media type has different data components
    media: Query<(EntityRef, &ItemUuid, &Media), Without<MediaFile>>,
    engine: Res<PatternEngine>,
    mut cmds: Commands,
) {
    // construct the structures that will head into the renaming engine
    // run the file path through the renaming engine
    // set the file path to the renamed path
    for (ent, media_id, mut file, _, file_info) in files {
        // @todo: update context entries + return new context (needed for file moving)
        let (media_info, _, _) = media.iter().find(|(_, id, _)| id.0 == media_id.0).unwrap();
        let mut context = Context::new();
        context.extend(file_info.get_context());

        let media_titles = media_info.get::<MediaTitles>().unwrap();
        let eng_title = media_titles.0.iter().find(|t|
            t.language == Some("en".to_string()) && t.traits.contains(TitleTrait::Official)
        ).unwrap();
        context.insert("media.title", eng_title.title.as_str());
        // how can i insert the file path into the context?
        // ^^ should i split it into series + season?
        // ^^ in that case, i'll need media relation data to track the series chain
        // ^^ how could i associate an item with a particular series

        // insert "output" items with their default values
        context.insert("file.name", file_info.title.clone());
        let root_path = settings.media_folders().iter().find(|f| f.id() == file.folder_id()).unwrap();
        let file_path = root_path.destination_path().iter()
            .map(|s| backend_types::renamer::Value::Str(s.to_str().unwrap().into()))
            .collect::<Vec<_>>();
        context.insert("file.path", file_path);
        context.insert("file.extension", file_info.file_extension.as_deref().unwrap_or(""));

        engine.render(&mut context);

        // @todo: support moving extras + image + other folders over
        // file path: root_path/media_series/media_name
        // file name: <file name>
        let file_path = context.get("file.path").unwrap()
            .as_array().unwrap()
            .iter().map(|v| v.as_string().unwrap())
            .collect::<std::path::PathBuf>();
        let file_path = file_path.join(context.get("file.name").unwrap().as_string().unwrap());

        // create any missing directories before moving file
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::rename(file.path(), &file_path).unwrap();
        *file.path_mut() = file_path;

        // we have finished processing the file
        cmds.entity(ent).remove::<UnprocessedFile>();
    }
}

fn retry_title(title: &str, retry_n: u8) -> Option<&str> {
    // if the title is empty, then the query will always fail
    if !title.is_empty() {
        match retry_n {
            1 => {
                // remove everything after last dash (ex. Yu-Gi-Oh! Kaï 11 ==- Dungeon Dice Monsters==)
                let last_dash = title.rfind('-').unwrap_or(title.len());
                Some(title[..last_dash].trim())
            }
            2 => {
                // strip trailing numbers (ex. Yu-Gi-Oh! Kaï ==11==)
                Some(title.trim_end_matches(|c: char| c.is_numeric()))
            }
            3 => {
                // strip non-latin characters (especially cjk characters) (ex. Yuru Camp △ Movie ==映画 ゆるキャン==△) (ex. **悲剧元凶的最强外道最终**BOSS**女王为民竭力**)
                Some(title.trim_matches(|c: char| matches!(unicode_general_category::get_general_category(c), unicode_general_category::GeneralCategory::OtherLetter)))
            }
            4 => {
                // strip trailing symbols, and everything not letters (ex. Yuru Camp △ Movie ==△==)
                Some(title.trim_end_matches(|c: char| !c.is_alphabetic()))
            }
            _ => None
        }
    } else { None }
}
