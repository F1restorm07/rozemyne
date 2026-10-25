use std::sync::Arc;
use std::time::Duration;

use arrow::array::{AsArray, DurationSecondBuilder, FixedSizeBinaryBuilder, Int16Builder, ListBuilder, RecordBatch, StringBuilder, StructBuilder, UInt8Builder, UInt32Builder};
use arrow::datatypes::{DataType, Field, Int16Type, Schema, TimeUnit, UInt8Type, UInt32Type};
use bevy_ecs::resource::Resource;
use derive_builder::Builder;
use enumset::EnumSet;
use num_enum::TryFromPrimitive;
use uuid::Uuid;

use crate::media::{AnimeFormat, FuzzyDate, Gender, MangaFormat, MediaAirSeason, MediaSource, MediaStatus, MediaTitle, MediaTitles, SourceId, SourceIds, TitleTrait};

#[derive(Resource)]
pub struct ArrowStorage(pub Vec<RecordBatch>);

// @todo: add source ids
// @todo: drop the `models` representation in `backend_core` for this one
// @todo: add relationships
#[derive(Debug, Clone, Builder)]
pub struct Anime {
    id: Uuid, // using a uuid here since bevy's entity ids are considered unstable acorss program runs
    format: AnimeFormat,
    #[builder(default)]
    air_season: Option<MediaAirSeason>,
    #[builder(default)]
    episodes: Option<u32>,
    #[builder(default)]
    episode_duration: Option<Duration>,
    #[builder(default)]
    description: Option<String>,
    #[builder(default)]
    start_date: Option<FuzzyDate>,
    #[builder(default)]
    end_date: Option<FuzzyDate>,
    source: MediaSource,
    status: MediaStatus,
    // titles and source ids are on here since they are 1-to-1 relationships (source ids + titles are only associated with a single media item)
    titles: MediaTitles,
    source_ids: SourceIds,
}

impl Anime {
    pub fn schema() -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::FixedSizeBinary(16), false),
            Field::new("format", DataType::UInt8, false),
            Field::new_struct("air_season", vec![
                Field::new("year", DataType::UInt32, false),
                Field::new("season", DataType::UInt8, false),
            ], true),
            Field::new("episodes", DataType::UInt32, true),
            Field::new("episode_duration", DataType::Duration(TimeUnit::Second), true),
            Field::new("description", DataType::Utf8, true),
            Field::new_struct("start_date", vec![
                Field::new("year", DataType::Int16, true),
                Field::new("month", DataType::UInt8, true),
                Field::new("day", DataType::UInt8, true),
            ], true),
            Field::new_struct("end_date", vec![
                Field::new("year", DataType::Int16, true),
                Field::new("month", DataType::UInt8, true),
                Field::new("day", DataType::UInt8, true),
            ], true),
            Field::new("source", DataType::UInt8, false),
            Field::new("status", DataType::UInt8, false),
            Field::new_list("titles", Field::new_list_field(DataType::Struct(vec![
                Field::new("title", DataType::Utf8, false),
                Field::new("language", DataType::Utf8, true),
                Field::new("traits", DataType::UInt8, false),
            ].into()), false), false),
            Field::new_list("source_ids", Field::new_list_field(DataType::Struct(vec![
                Field::new("source", DataType::Utf8, false),
                Field::new("id", DataType::UInt32, false),
            ].into()), false), false),
        ])
    }

    // converts rows of anime into the columnar `RecordBatch` format
    pub fn to_record_batch(anime: Vec<Anime>) -> RecordBatch {
        let mut id_builder = FixedSizeBinaryBuilder::with_capacity(anime.len(), 16);
        let mut format_builder = UInt8Builder::with_capacity(anime.len());
        let mut air_season_builder = StructBuilder::from_fields(vec![
            Field::new("year", DataType::UInt32, false),
            Field::new("season", DataType::UInt8, false),
        ], anime.len());
        let mut episodes_builder = UInt32Builder::with_capacity(anime.len());
        let mut episode_dur_builder = DurationSecondBuilder::with_capacity(anime.len());
        let mut desc_builder = StringBuilder::with_capacity(anime.len(), 0);
        let mut start_date_builder = StructBuilder::from_fields(vec![
            Field::new("year", DataType::Int16, true),
            Field::new("month", DataType::UInt8, true),
            Field::new("day", DataType::UInt8, true),
        ], anime.len());
        let mut end_date_builder = StructBuilder::from_fields(vec![
            Field::new("year", DataType::Int16, true),
            Field::new("month", DataType::UInt8, true),
            Field::new("day", DataType::UInt8, true),
        ], anime.len());
        let mut source_builder = UInt8Builder::with_capacity(anime.len());
        let mut status_builder = UInt8Builder::with_capacity(anime.len());
        let mut titles_builder = ListBuilder::new(StructBuilder::from_fields(vec![
            Field::new("title", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, true),
            Field::new("traits", DataType::UInt8, false),
        ], 0))
            .with_field(Field::new_list_field(DataType::Struct(vec![
                Field::new("title", DataType::Utf8, false),
                Field::new("language", DataType::Utf8, true),
                Field::new("traits", DataType::UInt8, false),
            ].into()), false));
        let mut source_ids_builder = ListBuilder::new(StructBuilder::from_fields(vec![
            Field::new("source", DataType::Utf8, false),
            Field::new("id", DataType::UInt32, false),
        ], 0))
            .with_field(Field::new_list_field(DataType::Struct(vec![
                Field::new("source", DataType::Utf8, false),
                Field::new("id", DataType::UInt32, false),
            ].into()), false));

        for anime in anime {
            id_builder.append_value(anime.id.into_bytes()).unwrap();
            format_builder.append_value(anime.format.into());

            if let Some(air_season) = anime.air_season {
                air_season_builder.field_builder::<UInt32Builder>(0).unwrap().append_value(air_season.year);
                air_season_builder.field_builder::<UInt8Builder>(1).unwrap().append_value(air_season.season.into());
                // append the elements to the struct itself (^^ above was to append the actual elements to the child arrays)
                air_season_builder.append(true);
            } else {
                air_season_builder.append_null();
            }
            if let Some(ep) = anime.episodes {
                episodes_builder.append_value(ep);
            } else {
                episodes_builder.append_null();
            }
            if let Some(dur) = anime.episode_duration {
                episode_dur_builder.append_value(dur.as_secs() as i64);
            } else {
                episode_dur_builder.append_null();
            }
            if let Some(desc) = anime.description {
                desc_builder.append_value(desc);
            } else {
                desc_builder.append_null();
            }
            if let Some(start_date) = anime.start_date {
                if let Some(y) = start_date.year {
                    start_date_builder.field_builder::<Int16Builder>(0).unwrap().append_value(y);
                } else { start_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null(); }
                if let Some(m) = start_date.month {
                    start_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_value(m);
                } else { start_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null(); }
                if let Some(d) = start_date.day {
                    start_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(d);
                } else { start_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null(); }
                start_date_builder.append(true);
            } else {
                start_date_builder.append_null();
            }
            if let Some(end_date) = anime.end_date {
                if let Some(y) = end_date.year {
                    end_date_builder.field_builder::<Int16Builder>(0).unwrap().append_value(y);
                } else { end_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null(); }
                if let Some(m) = end_date.month {
                    end_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_value(m);
                } else { end_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null(); }
                if let Some(d) = end_date.day {
                    end_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(d);
                } else { end_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null(); }
                end_date_builder.append(true);
            } else {
                end_date_builder.append_null();
            }

            source_builder.append_value(anime.source.into());
            status_builder.append_value(anime.status.into());

            let titles_inner_builder = titles_builder.values();
            for title in anime.titles.0 {
                titles_inner_builder.field_builder::<StringBuilder>(0).unwrap().append_value(title.title);
                if let Some(lan) = title.language {
                    titles_inner_builder.field_builder::<StringBuilder>(1).unwrap().append_value(lan);
                } else { titles_inner_builder.field_builder::<StringBuilder>(1).unwrap().append_null(); }
                titles_inner_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(title.traits.as_u8());
                titles_inner_builder.append(true);
            }
            titles_builder.append(true);

            let source_ids_inner_builder = source_ids_builder.values();
            for source_id in anime.source_ids.0 {
                source_ids_inner_builder.field_builder::<StringBuilder>(0).unwrap().append_value(source_id.source);
                source_ids_inner_builder.field_builder::<UInt32Builder>(1).unwrap().append_value(source_id.id);
                source_ids_inner_builder.append(true);
            }
            source_ids_builder.append(true);
        }

        RecordBatch::try_new(Arc::new(Anime::schema()), vec![
            Arc::new(id_builder.finish()),
            Arc::new(format_builder.finish()),
            Arc::new(air_season_builder.finish()),
            Arc::new(episodes_builder.finish()),
            Arc::new(episode_dur_builder.finish()),
            Arc::new(desc_builder.finish()),
            Arc::new(start_date_builder.finish()),
            Arc::new(end_date_builder.finish()),
            Arc::new(source_builder.finish()),
            Arc::new(status_builder.finish()),
            Arc::new(titles_builder.finish()),
            Arc::new(source_ids_builder.finish()),
        ]).expect("could not build record batch for anime")
    }
}

#[derive(Debug, Clone, Builder)]
pub struct Manga {
    pub id: Uuid,
    pub format: MangaFormat,
    #[builder(default)]
    pub chapters: Option<u32>,
    #[builder(default)]
    pub volumes: Option<u32>,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub start_date: Option<FuzzyDate>,
    #[builder(default)]
    pub end_date: Option<FuzzyDate>,
    pub source: MediaSource,
    pub status: MediaStatus,
    pub titles: MediaTitles,
    pub source_ids: SourceIds,
}

impl MangaBuilder {
    pub fn insert_titles(&mut self, titles: MediaTitles) {
        if let Some(media_titles) = self.titles.as_mut() {
            // @todo: check on the title itself, and insert any missing information
            let new_titles = titles.0.clone().into_iter().filter(|t| !media_titles.0.contains(t)).collect::<Vec<_>>();
            media_titles.0.extend_from_slice(&new_titles);
        } else { self.titles = Some(titles); }
    }
    pub fn insert_source_id(&mut self, source_id: crate::media::SourceId) {
        if let Some(media_source_ids) = self.source_ids.as_mut() {
            if !media_source_ids.0.contains(&source_id) {
                media_source_ids.0.push(source_id);
            }
        } else { self.source_ids = Some(SourceIds(vec![source_id])) }
    }
}

impl Manga {
    pub fn schema() -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::FixedSizeBinary(16), false),
            Field::new("format", DataType::UInt8, false),
            Field::new("chapters", DataType::UInt32, true),
            Field::new("volumes", DataType::UInt32, true),
            Field::new("description", DataType::Utf8, true),
            Field::new_struct("start_date", vec![
                Field::new("year", DataType::Int16, true),
                Field::new("month", DataType::UInt8, true),
                Field::new("day", DataType::UInt8, true),
            ], true),
            Field::new_struct("end_date", vec![
                Field::new("year", DataType::Int16, true),
                Field::new("month", DataType::UInt8, true),
                Field::new("day", DataType::UInt8, true),
            ], true),
            Field::new("source", DataType::UInt8, false),
            Field::new("status", DataType::UInt8, false),
            // create a list field containing every title for a given manga
            // now, to match the behavior we want, we create a specialized list field with the name "item"
            // then fill it with the struct fields we want
            Field::new_list("titles", Field::new_list_field(DataType::Struct(vec![
                Field::new("title", DataType::Utf8, false),
                Field::new("language", DataType::Utf8, true),
                Field::new("traits", DataType::UInt8, false),
            ].into()), false), false),
            Field::new_list("source_ids", Field::new_list_field(DataType::Struct(vec![
                Field::new("source", DataType::Utf8, false),
                Field::new("id", DataType::UInt32, false),
            ].into()), false), false),
        ])
    }

    pub fn to_record_batch(manga: Vec<Manga>) -> RecordBatch {
        let mut id_builder = FixedSizeBinaryBuilder::with_capacity(manga.len(), 16);
        let mut format_builder = UInt8Builder::with_capacity(manga.len());
        let mut chapters_builder = UInt32Builder::with_capacity(manga.len());
        let mut volumes_builder = UInt32Builder::with_capacity(manga.len());
        let mut desc_builder = StringBuilder::with_capacity(manga.len(), 0);
        let mut start_date_builder = StructBuilder::from_fields(vec![
            Field::new("year", DataType::Int16, true),
            Field::new("month", DataType::UInt8, true),
            Field::new("day", DataType::UInt8, true),
        ], manga.len());
        let mut end_date_builder = StructBuilder::from_fields(vec![
            Field::new("year", DataType::Int16, true),
            Field::new("month", DataType::UInt8, true),
            Field::new("day", DataType::UInt8, true),
        ], manga.len());
        let mut source_builder = UInt8Builder::with_capacity(manga.len());
        let mut status_builder = UInt8Builder::with_capacity(manga.len());
        let mut titles_builder = ListBuilder::new(StructBuilder::from_fields(vec![
            Field::new("title", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, true),
            Field::new("traits", DataType::UInt8, false),
        ], 0))
            // override the default field to be non-nullable
            .with_field(Field::new_list_field(DataType::Struct(vec![
                Field::new("title", DataType::Utf8, false),
                Field::new("language", DataType::Utf8, true),
                Field::new("traits", DataType::UInt8, false),
            ].into()), false));
        let mut source_ids_builder = ListBuilder::new(StructBuilder::from_fields(vec![
            Field::new("source", DataType::Utf8, false),
            Field::new("id", DataType::UInt32, false),
        ], 0))
            .with_field(Field::new_list_field(DataType::Struct(vec![
                Field::new("source", DataType::Utf8, false),
                Field::new("id", DataType::UInt32, false),
            ].into()), false));

        for manga in manga {
            id_builder.append_value(manga.id.into_bytes()).unwrap();
            format_builder.append_value(manga.format.into());

            if let Some(ch) = manga.chapters {
                chapters_builder.append_value(ch);
            } else {
                chapters_builder.append_null();
            }
            if let Some(vol) = manga.volumes {
                volumes_builder.append_value(vol);
            } else {
                volumes_builder.append_null();
            }
            if let Some(desc) = manga.description {
                desc_builder.append_value(desc);
            } else {
                desc_builder.append_null();
            }
            if let Some(start_date) = manga.start_date {
                if let Some(y) = start_date.year {
                    start_date_builder.field_builder::<Int16Builder>(0).unwrap().append_value(y);
                } else { start_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null(); }
                if let Some(m) = start_date.month {
                    start_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_value(m);
                } else { start_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null(); }
                if let Some(d) = start_date.day {
                    start_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(d);
                } else { start_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null(); }
                start_date_builder.append(true);
            } else {
                // always append to *every* child field before appending to the struct/list
                start_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null();
                start_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null();
                start_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null();
                start_date_builder.append_null();
            }
            if let Some(end_date) = manga.end_date {
                if let Some(y) = end_date.year {
                    end_date_builder.field_builder::<Int16Builder>(0).unwrap().append_value(y);
                } else { end_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null(); }
                if let Some(m) = end_date.month {
                    end_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_value(m);
                } else { end_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null(); }
                if let Some(d) = end_date.day {
                    end_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(d);
                } else { end_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null(); }
                end_date_builder.append(true);
            } else {
                end_date_builder.field_builder::<Int16Builder>(0).unwrap().append_null();
                end_date_builder.field_builder::<UInt8Builder>(1).unwrap().append_null();
                end_date_builder.field_builder::<UInt8Builder>(2).unwrap().append_null();
                end_date_builder.append_null();
            }

            source_builder.append_value(manga.source.into());
            status_builder.append_value(manga.status.into());

            let titles_inner_builder = titles_builder.values();
            for title in manga.titles.0 {
                titles_inner_builder.field_builder::<StringBuilder>(0).unwrap().append_value(title.title);
                if let Some(lan) = title.language {
                    titles_inner_builder.field_builder::<StringBuilder>(1).unwrap().append_value(lan);
                } else { titles_inner_builder.field_builder::<StringBuilder>(1).unwrap().append_null(); }
                titles_inner_builder.field_builder::<UInt8Builder>(2).unwrap().append_value(title.traits.as_u8());
                titles_inner_builder.append(true);
            }
            // append an offset value into the list buffer (breaks the child array into sections)
            titles_builder.append(true);

            let source_ids_inner_builder = source_ids_builder.values();
            for source_id in manga.source_ids.0 {
                source_ids_inner_builder.field_builder::<StringBuilder>(0).unwrap().append_value(source_id.source);
                source_ids_inner_builder.field_builder::<UInt32Builder>(1).unwrap().append_value(source_id.id);
                source_ids_inner_builder.append(true);
            }
            source_ids_builder.append(true);
        }

        RecordBatch::try_new(Arc::new(Manga::schema()), vec![
            Arc::new(id_builder.finish()),
            Arc::new(format_builder.finish()),
            Arc::new(chapters_builder.finish()),
            Arc::new(volumes_builder.finish()),
            Arc::new(desc_builder.finish()),
            Arc::new(start_date_builder.finish()),
            Arc::new(end_date_builder.finish()),
            Arc::new(source_builder.finish()),
            Arc::new(status_builder.finish()),
            Arc::new(titles_builder.finish()),
            Arc::new(source_ids_builder.finish()),
        ]).expect("could not build record batch for anime")
    }

    pub fn from_record_batch(records: RecordBatch) -> Vec<Self> {
        let records = records.with_schema(Arc::new(Manga::schema())).unwrap();
        // convert the column representation into rows for command

        let columns = records.columns();
        let id = columns[0].as_fixed_size_binary().iter();
        let format = columns[1].as_primitive::<UInt8Type>().iter();
        let chapters = columns[2].as_primitive::<UInt32Type>().iter();
        let volumes = columns[3].as_primitive::<UInt32Type>().iter();
        let description = columns[4].as_string::<i32>().iter();
        let source = columns[7].as_primitive::<UInt8Type>().iter();
        let status = columns[8].as_primitive::<UInt8Type>().iter();

        let start_date = columns[5].as_struct().columns();
        let start_date = itertools::izip!(
            start_date[0].as_primitive::<Int16Type>().iter(),
            start_date[1].as_primitive::<UInt8Type>().iter(),
            start_date[2].as_primitive::<UInt8Type>().iter(),
        ).map(|(year, month, day)| {
            if year.is_some() || month.is_some() || day.is_some() {
                Some(FuzzyDate {
                    year,
                    month,
                    day,
                })
            } else { None }
        });
        let end_date = columns[6].as_struct().columns();
        let end_date = itertools::izip!(
            end_date[0].as_primitive::<Int16Type>().iter(),
            end_date[1].as_primitive::<UInt8Type>().iter(),
            end_date[2].as_primitive::<UInt8Type>().iter(),
        ).map(|(year, month, day)| {
            if year.is_some() || month.is_some() || day.is_some() {
                Some(FuzzyDate {
                    year,
                    month,
                    day,
                })
            } else { None }
        });

        let titles = columns[9].as_list::<i32>().iter().map(|t| {
            let t = t.unwrap();
            let titles = t.as_struct().columns();
            itertools::izip!(
                titles[0].as_string::<i32>().iter(),
                titles[1].as_string::<i32>().iter(),
                titles[2].as_primitive::<UInt8Type>().iter(),
            ).map(|(t, lang, traits)| {
                MediaTitle {
                    title: t.unwrap().to_string(),
                    language: lang.map(|l| l.to_string()),
                    traits: EnumSet::from_u8(traits.unwrap())
                }
            }).collect::<Vec<_>>()
        });
        let source_ids = columns[10].as_list::<i32>().iter().map(|i| {
            let i = i.unwrap();
            let ids = i.as_struct().columns();
            itertools::izip!(
                ids[0].as_string::<i32>().iter(),
                ids[1].as_primitive::<UInt32Type>().iter(),
            ).map(|(source, id)| {
                SourceId {
                    source: source.unwrap().to_string(),
                    id: id.unwrap(),
                }
            }).collect()
        });

        let iterator = itertools::izip!(
            id, format,
            chapters, volumes, description,
            start_date, end_date,
            source, status,
            titles, source_ids,
        );

        iterator.map(|(id, format, chapters, volumes, desc, start_date, end_date, source, status, titles, source_ids)| {
            let mut manga = MangaBuilder::default();
            manga.id(Uuid::from_bytes(*id.unwrap().as_array().unwrap()))
                .format(MangaFormat::try_from_primitive(format.unwrap()).unwrap())
                .chapters(chapters)
                .volumes(volumes)
                .description(desc.map(|d| d.to_string()))
                .start_date(start_date)
                .end_date(end_date)
                .source(MediaSource::try_from_primitive(source.unwrap()).unwrap())
                .status(MediaStatus::try_from_primitive(status.unwrap()).unwrap())
                .titles(MediaTitles(titles))
                .source_ids(SourceIds(source_ids))
                .build().unwrap()
        }).collect()
    }
}

pub struct StaffMember {
    id: Uuid,
    name: String,
    desc: Option<String>,
    age: Option<u32>,
    birth_date: Option<FuzzyDate>,
    gender: Option<Gender>,
    hometown: Option<String>,
    years_active: Option<String>,
    source_ids: SourceIds,
}
