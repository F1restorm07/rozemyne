use std::fs::File;
use std::time::Duration;

use arrow::array::{ArrayIter, AsArray};
use backend_types::media::data_source::Media;
use backend_types::media::{AnimeFormat, ItemUuid, MangaFormat, MediaAirDate, MediaAirSeason, MediaChapterAmount, MediaDescription, MediaEpisodeAmount, MediaEpisodeDuration, MediaSource, MediaStatus, MediaTitles, MediaType, MediaVolumeAmount, SourceIds};
use backend_types::schemas::storage::{Anime, AnimeBuilder, Manga, MangaBuilder};
use bevy_app::{AppExit, FixedMainScheduleOrder, MainScheduleOrder};
use bevy_ecs::change_detection::Res;
use bevy_ecs::schedule::common_conditions::on_message;
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleLabel, SystemCondition, SystemSet};
use bevy_ecs::system::{Commands, Populated, Query};
use bevy_time::common_conditions::on_timer;
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ArrowReaderBuilder;
use parquet::basic::ZstdLevel;
use parquet::file::properties::WriterProperties;

use crate::configuration::Settings;

// should i collapse the intermediate step of building `Anime`
// and convert directly into the arrow representation?
// @todo: add relationships here?? (i'll need to store the relationships in a separate file, but should i store the targets here?)
// ^^ type would be Map<MediaUuid, List<some sort of relationship id>>
// ^^ can i use the relationship type to register the targets onto the relevent entities (i can with my setup, but i need the entity ids -> can i get the ids with the MediaUuid?)
pub fn archive_anime(
    anime: Populated<(
        &ItemUuid, &AnimeFormat, Option<&MediaAirSeason>, Option<&MediaEpisodeAmount>, Option<&MediaEpisodeDuration>,
        Option<&MediaDescription>, &MediaAirDate, &MediaSource, &MediaStatus, &SourceIds, &MediaTitles,
    )>,
    settings: Res<Settings>,
) {
    let anime = anime.iter().map(|(id, format, air_season, episodes, ep_dur, desc, air_date, source, status, source_ids, titles)| {
        AnimeBuilder::default()
            .id(id.0)
            .format(format.clone())
            .air_season(air_season.cloned())
            .episodes(episodes.map(|e| e.0))
            .episode_duration(ep_dur.map(|d| Duration::from_mins(d.0 as u64)))
            .description(desc.map(|d| d.0.clone()))
            .start_date(air_date.start.clone())
            .end_date(air_date.end.clone())
            .source(source.clone())
            .status(status.clone())
            .source_ids(source_ids.clone())
            .titles(titles.clone())
            .build().unwrap()
    }).collect::<Vec<_>>();
    let record_batch = Anime::to_record_batch(anime);

    let path = settings.as_ref().archive_path().join("anime.parquet");
    let file = File::create(path).unwrap();
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::ZSTD(ZstdLevel::default()))
        .build();
    let mut writer = ArrowWriter::try_new(file, record_batch.schema(), Some(props)).unwrap();
    writer.write(&record_batch).unwrap();
    writer.close().unwrap();
}

pub fn archive_manga(
    // manga: Populated<(
    manga: Query<(
        &ItemUuid, &MangaFormat, Option<&MediaChapterAmount>, Option<&MediaVolumeAmount>,
        Option<&MediaDescription>, &MediaAirDate, &MediaSource, &MediaStatus, &SourceIds, &MediaTitles,
    )>,
    settings: Res<Settings>,
) {
    // println!("archiving manga: {}");
    let manga = manga.iter().map(|(id, format, ch, vol, desc, air_date, source, status, source_ids, titles)| {
        MangaBuilder::default()
            .id(id.0)
            .format(format.clone())
            .chapters(ch.map(|c| c.0))
            .volumes(vol.map(|v| v.0))
            .description(desc.map(|d| d.0.clone()))
            .start_date(air_date.start.clone())
            .end_date(air_date.end.clone())
            .source(source.clone())
            .status(status.clone())
            .source_ids(source_ids.clone())
            .titles(titles.clone())
            .build().unwrap()
    }).collect::<Vec<_>>();
    let record_batch = Manga::to_record_batch(manga);

    let path = settings.as_ref().archive_path().join("manga.parquet");
    let file = File::create(path).unwrap();
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::ZSTD(ZstdLevel::default()))
        .build();
    let mut writer = ArrowWriter::try_new(file, record_batch.schema(), Some(props)).unwrap();
    writer.write(&record_batch).unwrap();
    writer.close().unwrap();
}

// what should i do if the file does not exist yet?
// ^^ probably nothing, since then no media exists
pub fn build_manga(settings: Res<Settings>, mut cmds: Commands) {
    if let Ok(manga_file) = File::open(settings.archive_path().join("manga.parquet")) {
        let builder = ArrowReaderBuilder::try_new(manga_file).unwrap();
        let mut reader = builder.build().unwrap();
        let record_batch = reader.next().unwrap().unwrap();

        for manga in Manga::from_record_batch(record_batch) {
            let mut ent = cmds.spawn((
                Media(MediaType::Manga), ItemUuid(manga.id),
                MediaAirDate { start: manga.start_date, end: manga.end_date },
                manga.format, manga.source, manga.status, manga.titles, manga.source_ids
            ));

            if let Some(ch) = manga.chapters {
                ent.insert(MediaChapterAmount(ch));
            }
            if let Some(vol) = manga.volumes {
                ent.insert(MediaVolumeAmount(vol));
            }
            if let Some(desc) = manga.description {
                ent.insert(MediaDescription(desc));
            }
        }
    }
}

pub struct ArchivalPlugin;

impl bevy_app::Plugin for ArchivalPlugin {
    // @todo: add data retrieval process on `Startup`
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(bevy_app::Startup, (
            build_manga
        ));
        let settings = app.world().get_resource::<Settings>().unwrap();
        // tick the archival timers at a fixed interval
        // run the schedule on a timer, or when the app is shutting down
        app.add_systems(bevy_app::Last, (
            archive_anime, archive_manga,
            ).run_if(on_timer(settings.archive_frequency())
                .or_else(on_message::<AppExit>))
        );
    }
}
