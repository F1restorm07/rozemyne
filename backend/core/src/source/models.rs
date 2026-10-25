use std::collections::BTreeMap;

use backend_types::db::connection::DbConnection;
use backend_types::db::{AnimeFormat as DbAnimeFormat, EpisodeType, FuzzyDate, Gender, MangaFormat as DbMangaFormat, MediaSeason, Season, Status};
use backend_types::media::{AirSeason, AnimeFormat, MangaFormat, MediaAirDate, MediaAirSeason, MediaChapterAmount, MediaDescription, MediaEpisodeAmount, MediaEpisodeDuration, MediaSource, MediaStatus, MediaTitles, MediaType, MediaVolumeAmount, SourceId, SourceIds};
use backend_types::media::data_source::{DataSourceName, Media, MediaDataResponse, MediaInfoQuery, MediaRequestUuid, MediaSourceId};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::query::{Or, With, Without};
use bevy_ecs::resource::IsResource;
use bevy_ecs::system::{Commands, Query, ResMut};
use bevy_ecs::world::EntityRef;
use derive_builder::Builder;
use diesel::dsl::{Filter, Is};
use diesel::expression::SqlLiteral;
use diesel::sql_types::Bool;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use enumset::{EnumSet, EnumSetType};
use uuid::Uuid;

use crate::database::models::{NewRelation, NewSourceId, NewTitle, SourceId as DbSourceId};
use crate::database::models::episodic_media::{NewEpisodicMedia, NewManga};
use crate::database::NewAnime;

// based on: https://anonymousoverflow.catsarch.com/questions/4253960/sql-how-to-properly-check-if-a-record-exists
pub fn check_for_source_ids(ids: impl IntoIterator<Item = SourceId>) -> Filter<backend_types::schemas::db::source_ids::table, SqlLiteral<Bool>> {
    // check if any source id in the given set exists in the database already
    // basic idea:
    // SELECT 1 FROM <table> WHERE <key> = <value>
    //
    // execution:
    // SELECT * FROM <table> WHERE (<nested or filters>) LIMIT 1
    // nested or filters:
    // (source = <source 1> AND source_id = <id 1>) OR (source = <source 2> AND source_id = <id 2>) ... **chained together**

    // @todo: replace this if i can manage to find a dsl-based method for representing this
    let filter = ids.into_iter()
        // unwrapping the source id here as, hopefully, we have data sources which have ids for all their data
        .map(|s| format!("(source = '{}' AND source_id = {})", s.source, s.id.unwrap()))
        .collect::<Vec<_>>()
        .join(" OR ");
    let filter = diesel::dsl::sql::<diesel::sql_types::Bool>(&filter);
    backend_types::schemas::db::source_ids::table
        .filter(filter)
}

// can i break this down into components???? -> ehhhhh, maybe
// cleaned and processed anime data (transferable to database tables with conversion)
#[derive(Debug, Clone, Builder)]
struct Anime {
    format: AnimeFormat,
    #[builder(default)]
    air_season: Option<MediaAirSeason>,
    #[builder(default)]
    num_episodes: Option<i32>,
    #[builder(default)]
    episode_duration: Option<i32>,
    // episodes: Vec<Episode>, // not supported yet (need AniDB online to test)
    #[builder(default)]
    desc: Option<String>,
    air_date: MediaAirDate,
    source: MediaSource,
    status: MediaStatus,
    titles: MediaTitles,
    source_ids: SourceIds,
    // no `MediaRelations` here since we attach characters, staff, relaionts, etc in their own separate functions
    // ^^^ if relations are included in the `Info` response, then the data source must emit a new response with the corresponding
    // response date + type (see MangaBaka's info request for more info)
}

// @todo: add a `MediaRegistry` resource
pub fn anime_info_builder(
    resps: Query<
        (EntityRef, &MediaRequestUuid, &MediaType),
        With<MediaDataResponse<MediaInfoQuery>>,
    >,
    clients: Query<(Entity, &DataSourceName)>,
    mut cmds: Commands,
) {
    let anime_resps = resps.iter()
        .filter(|(_, _, t)| **t == MediaType::Anime)
        .fold(BTreeMap::<MediaRequestUuid, Vec<EntityRef>>::new(), |mut acc, (ent, id, _)| {
            acc.entry(id.clone()).or_default().push(ent);
            acc
        });

    for (req_key, resps) in anime_resps.iter() {
        // let mut anime_entry = media_registry.anime.entry(req_key.clone()).or_insert(AnimeBuilder::default());
        let mut anime_entry = AnimeBuilder::default();

        for resp in resps {
            if let Some(format) = resp.get::<AnimeFormat>() {
                if anime_entry.format.is_none() {
                    anime_entry.format(format.clone());
                }
            }
            if let Some(air_season) = resp.get::<MediaAirSeason>() {
                let season = match air_season.season {
                    AirSeason::Winter => Season::Winter,
                    AirSeason::Spring => Season::Spring,
                    AirSeason::Summer => Season::Summer,
                    AirSeason::Fall => Season::Fall,
                };
                if anime_entry.air_season.is_none() {
                    anime_entry
                        .air_season(Some(air_season.clone()));
                }
            }
            if let Some(episodes) = resp.get::<MediaEpisodeAmount>() {
                let episodes = episodes.0;
                if anime_entry.num_episodes.is_none() {
                    anime_entry.num_episodes(Some(episodes as i32));
                }
            }
            if let Some(episode_dur) = resp.get::<MediaEpisodeDuration>() {
                let episode_dur = episode_dur.0;
                if anime_entry.episode_duration.is_none() {
                    anime_entry.episode_duration(Some(episode_dur as i32));
                }
            }
            if let Some(desc) = resp.get::<MediaDescription>() {
                if anime_entry.desc.is_none() {
                    anime_entry.desc(Some(desc.0.clone()));
                }
            }
            if let Some(air_date) = resp.get::<MediaAirDate>() {
                if anime_entry.air_date.is_none() {
                    anime_entry.air_date(air_date.clone());
                }
            }
            if let Some(source) = resp.get::<MediaSource>() {
                if anime_entry.source.is_none() {
                    anime_entry.source(source.clone());
                }
            }
            if let Some(status) = resp.get::<MediaStatus>() {
                if anime_entry.status.is_none() {
                    anime_entry.status(status.clone());
                }
            }
            if let Some(source_id) = resp.get::<MediaSourceId>() {
                let source_id = source_id.0;
                let media_client_name = clients
                    .get(resp.get::<ChildOf>().unwrap().parent()).unwrap().1.0.clone();

                if anime_entry.source_ids.is_none() {
                    anime_entry.source_ids(SourceIds(vec![SourceId { source: media_client_name, id: Some(source_id) }]));
                } else if let Some(ref mut source_ids) = anime_entry.source_ids
                    && !source_ids.0.contains(&SourceId { source: media_client_name.clone(), id: Some(source_id) }) {
                    source_ids.0.push(SourceId { source: media_client_name, id: Some(source_id) });
                }
            }
            if let Some(titles) = resp.get::<MediaTitles>() {
                if let Some(anime_titles) = anime_entry.titles.as_mut() {
                    // let new_titles = titles.0.iter().filter(|t| !anime_titles.0.iter().any(|a| a.title == t.title));
                    let new_titles = titles.0.clone().into_iter().filter(|t| !anime_titles.0.contains(t)).collect::<Vec<_>>();
                    anime_titles.0.extend_from_slice(&new_titles);
                } else { anime_entry.titles(titles.clone()); }
            }

            // drop from `World`, this has been fully processed and is not needed anymore
            cmds.entity(resp.id()).despawn();
        }

        let anime = anime_entry.build().expect("could not build anime data structure");
        let mut anime_ent = cmds.spawn((
            req_key.clone(), Media(MediaType::Anime), anime.format, anime.source, anime.status,
            anime.titles, anime.source_ids,
        ));

        if let Some(air_season) = anime.air_season {
            anime_ent.insert(air_season);
        }
        if let Some(num_episodes) = anime.num_episodes {
            anime_ent.insert(MediaEpisodeAmount(num_episodes as u32));
        }
        if let Some(episode_dur) = anime.episode_duration {
            anime_ent.insert(MediaEpisodeDuration(episode_dur as u32));
        }
        if let Some(desc) = anime.desc {
            anime_ent.insert(MediaDescription(desc));
        }
    }
}

pub fn insert_anime_into_database(
    anime: Query<(
        EntityRef, &Media, &MediaRequestUuid,
        &AnimeFormat, &MediaSource, &MediaStatus, &MediaAirDate, &MediaTitles,
        &SourceIds
    )>,
    db_conn: ResMut<DbConnection>,
) {
    let mut db_conn = db_conn.0.get().unwrap();
    for (ent, _, media_id, format, source, status, air_date, titles, source_ids) in anime.iter().filter(|a| a.1.0 == MediaType::Anime) {
            db_conn.transaction(|c| {
                // have we already inserted the media into the database?
                if check_for_source_ids(source_ids.0.clone())
                    .load::<crate::database::models::SourceId>(c)
                    .is_ok_and(|r| r.is_empty())
                {
                    diesel::insert_into(backend_types::schemas::db::anime::table)
                        .values(&NewAnime {
                            id: media_id.0.as_bytes(),
                            format: backend_types::db::AnimeFormat::from(format.clone()),
                            air_season: ent.get::<MediaAirSeason>().map(|s| backend_types::db::MediaSeason {
                                season: backend_types::db::Season::from(s.season.clone()),
                                year: s.year as i16
                            }),
                            episodes: ent.get::<MediaEpisodeAmount>().map(|e| e.0 as i32),
                            episode_length: ent.get::<MediaEpisodeDuration>().map(|e| e.0 as i32),
                            description: ent.get::<MediaDescription>().map(|d| d.0.clone()),
                            start_date: air_date.start.clone().map(|a| a.into()),
                            end_date: air_date.end.clone().map(|a| a.into()),
                            source: backend_types::db::Source::from(source.clone()),
                            status: backend_types::db::Status::from(status.clone()),
                        })
                        .execute(c)?;

                     // double check titles table for our media id (already checked for source id existence so it *should* be fine)
                    if backend_types::schemas::db::titles::table
                        .select(backend_types::schemas::db::titles::media_id)
                        .filter(backend_types::schemas::db::titles::media_id.eq(media_id.0.as_bytes()))
                        .load::<Vec<u8>>(c).is_ok_and(|r| r.is_empty())
                    {
                        titles.0.iter().map(|t| NewTitle {
                            media_id: media_id.0.as_bytes(),
                            media_type: backend_types::db::MediaType::Anime,
                            title: t.title.clone(),
                            language: t.language.clone(),
                            traits: t.traits.as_u32() as i32,
                         })
                        .for_each(|t| {
                            diesel::insert_into(backend_types::schemas::db::titles::table)
                                .values(t)
                                .execute(c)
                                .unwrap();
                        });
                    }

                    // @todo: relations, characters, staff, studios, genres, source ids
                    let source_ids = source_ids.0
                        .iter().map(|s| NewSourceId {
                            item_id: media_id.0.as_bytes(),
                            item_type: backend_types::db::ItemType::Manga,
                            source: s.source.clone(),
                            source_id: s.id.unwrap() as i32,
                        })
                        .for_each(|s| {
                            diesel::insert_into(backend_types::schemas::db::source_ids::table)
                                .values(s)
                                .execute(c)
                                .unwrap();
                        });
                }
                Ok::<(), diesel::result::Error>(())
            });
    }
}

//             let relations = self
//                 .ep_media
//                 .relations
//                 .iter()
//                 .map(|r| NewRelation {
//                     source_id: id.as_bytes(),
//                     source_type: MediaType::Anime.into(),
//                     // check and see if the relation is already in the database
//                     // the `[u8; 16]` in the type is for this (returns an ownership error otherwise)
//                     target_id: check_for_source_ids(vec![(r.target_source.clone(), r.target_id)])
//                         .select(SourceId::as_select())
//                         .get_result::<SourceId>(c)
//                         .map(|res| *res.item_id.as_array().unwrap())
//                         // what happens when we add the actual entry later?
//                         // should we check the source id table then?
//                         .unwrap_or(*uuid::Uuid::now_v7().as_bytes()),
//                     target_type: r.target_type.into(),
//                     rel_type: r.rel_type,
//                 })
//                 .collect::<Vec<_>>();

// @todo: hook into an api like ComicVine or RanobeDB to access individual volume info
// @todo: add volumes once i settle on a representation
#[derive(Debug, Clone, Builder)]
struct Manga {
    format: MangaFormat,
    #[builder(default)]
    chapters: Option<u32>,
    #[builder(default)]
    volume_amt: Option<u32>,
    #[builder(default)]
    desc: Option<String>,
    serialize_date: MediaAirDate,
    source: MediaSource,
    status: MediaStatus,
    titles: MediaTitles,
    source_ids: SourceIds,
}

pub fn manga_info_builder(
    resps: Query<
        (EntityRef, &MediaRequestUuid, &MediaType),
        With<MediaDataResponse<MediaInfoQuery>>,
    >,
    clients: Query<(Entity, &DataSourceName)>,
    mut cmds: Commands,
) {
    let manga_resps = resps.iter()
        .filter(|(_, _, t)| **t == MediaType::Manga)
        .fold(BTreeMap::<MediaRequestUuid, Vec<EntityRef>>::new(), |mut acc, (ent, id, _)| {
            acc.entry(id.clone()).or_default().push(ent);
            acc
        });

    for (req_key, resps) in manga_resps {
        let mut manga_entry = MangaBuilder::default();

        for resp in resps {
            if let Some(format) = resp.get::<MangaFormat>() {
                if manga_entry.format.is_none() {
                    manga_entry.format(format.clone());
                }
            }
            if let Some(volume_amt) = resp.get::<MediaVolumeAmount>() {
                if manga_entry.volume_amt.is_none() {
                    manga_entry.volume_amt(Some(volume_amt.0));
                }
            }
            if let Some(chapters) = resp.get::<MediaChapterAmount>() {
                if manga_entry.chapters.is_none() {
                    manga_entry.chapters(Some(chapters.0));
                }
            }
            // vvvv same as manga -> refactor into separate function
            if let Some(desc) = resp.get::<MediaDescription>() {
                if manga_entry.desc.is_none() {
                    manga_entry.desc(Some(desc.0.clone()));
                }
            }
            if let Some(ser_date) = resp.get::<MediaAirDate>() {
                if manga_entry.serialize_date.is_none() {
                    manga_entry.serialize_date(ser_date.clone());
                }
            }
            if let Some(source) = resp.get::<MediaSource>() {
                if manga_entry.source.is_none() {
                    manga_entry.source(source.clone());
                }
            }
            if let Some(status) = resp.get::<MediaStatus>() {
                if manga_entry.status.is_none() {
                    manga_entry.status(status.clone());
                }
            }
            if let Some(source_id) = resp.get::<MediaSourceId>() {
                let source_id = source_id.0;
                let media_client_name = clients
                    .get(resp.get::<ChildOf>().unwrap().parent()).unwrap().1.0.clone();

                if manga_entry.source_ids.is_none() {
                    manga_entry.source_ids(SourceIds(vec![SourceId { source: media_client_name, id: Some(source_id) }]));
                } else if let Some(ref mut source_ids) = manga_entry.source_ids
                    && !source_ids.0.contains(&SourceId { source: media_client_name.clone(), id: Some(source_id) }) {
                    source_ids.0.push(SourceId { source: media_client_name, id: Some(source_id) });
                }
            }
            if let Some(titles) = resp.get::<MediaTitles>() {
                // @todo: find a better checking method here
                if let Some(manga_titles) = manga_entry.titles.as_mut() {
                    // let new_titles = titles.0.iter().filter(|t| !manga_titles.0.iter().any(|a| a.title == t.title));
                    let new_titles = titles.0.clone().into_iter().filter(|t| !manga_titles.0.contains(t)).collect::<Vec<_>>();
                    manga_titles.0.extend_from_slice(&new_titles);
                } else { manga_entry.titles(titles.clone()); }
            }

            cmds.entity(resp.id()).despawn();
        }

        let manga = manga_entry.build().expect("could not build manga data structure");
        let mut manga_ent = cmds.spawn((
            req_key.clone(), Media(MediaType::Manga), manga.format, manga.source, manga.status,
            manga.serialize_date, manga.titles, manga.source_ids,
        ));

        if let Some(volume_amt) = manga.volume_amt {
            manga_ent.insert(MediaVolumeAmount(volume_amt as u32));
        }
        if let Some(chapters) = manga.chapters {
            manga_ent.insert(MediaChapterAmount(chapters as u32));
        }
        if let Some(desc) = manga.desc {
            manga_ent.insert(MediaDescription(desc));
        }
    }
}

pub fn insert_manga_into_database(
    manga: Query<(
        EntityRef, &Media, &MediaRequestUuid,
        &MangaFormat, &MediaSource, &MediaStatus, &MediaAirDate, &MediaTitles,
        &SourceIds
    ), Without<IsResource>>,
    // vv this resource access conflicts with the above ^^^ query due to `EntityRef` i think?
    // (`EntityRef` gets the *entire* component for *all* components regardless of what components are in the archetype)
    db_conn: ResMut<DbConnection>,
) {
    let mut db_conn = db_conn.0.get().unwrap();
    for (ent, _, media_id, format, source, status, air_date, titles, source_ids) in manga.iter().filter(|a| a.1.0 == MediaType::Manga) {
            db_conn.transaction(|c| {
                // have we already inserted the media into the database?
                // @todo: when inserting, if the media is already in the database, but *not* in any media table (use the accompanying id to check),
                // insert the media using the preexisting id found in the source id table (omit the insert into the source id table)
                if check_for_source_ids(source_ids.0.clone())
                    .load::<crate::database::models::SourceId>(c)
                    .is_ok_and(|r| r.is_empty())
                {
                    diesel::insert_into(backend_types::schemas::db::manga::table)
                        .values(&NewManga {
                            id: media_id.0.as_bytes(),
                            format: backend_types::db::MangaFormat::from(format.clone()),
                            chapters: ent.get::<MediaChapterAmount>().map(|e| e.0 as i32),
                            volumes: ent.get::<MediaVolumeAmount>().map(|d| d.0 as i32),
                            description: ent.get::<MediaDescription>().map(|d| d.0.clone()),
                            start_date: air_date.start.clone().map(|a| a.into()),
                            end_date: air_date.end.clone().map(|a| a.into()),
                            source: backend_types::db::Source::from(source.clone()),
                            status: backend_types::db::Status::from(status.clone()),
                        })
                        .execute(c)?;

                     // double check titles table for our media id (already checked for source id existence so it *should* be fine)
                    if backend_types::schemas::db::titles::table
                        .select(backend_types::schemas::db::titles::media_id)
                        .filter(backend_types::schemas::db::titles::media_id.eq(media_id.0.as_bytes()))
                        .load::<Vec<u8>>(c).is_ok_and(|r| r.is_empty())
                    {
                        titles.0.iter().map(|t| NewTitle {
                            media_id: media_id.0.as_bytes(),
                            media_type: backend_types::db::MediaType::Anime,
                            title: t.title.clone(),
                            language: t.language.clone(),
                            traits: t.traits.as_u32() as i32,
                         })
                        .for_each(|t| {
                            diesel::insert_into(backend_types::schemas::db::titles::table)
                                .values(t)
                                .execute(c)
                                .unwrap();
                        });
                    }

                    // @todo: relations, characters, staff, studios, genres, source ids
                    let source_ids = source_ids.0
                        .iter().map(|s| NewSourceId {
                            item_id: media_id.0.as_bytes(),
                            item_type: backend_types::db::ItemType::Manga,
                            source: s.source.clone(),
                            source_id: s.id.unwrap() as i32,
                        })
                        .for_each(|s| {
                            diesel::insert_into(backend_types::schemas::db::source_ids::table)
                                .values(s)
                                .execute(c)
                                .unwrap();
                        });
                }
                Ok::<(), diesel::result::Error>(())
            });
    }
}
