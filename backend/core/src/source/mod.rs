use std::collections::BTreeMap;

use backend_types::cli::CliInternalQuery;
use backend_types::media::query::MediaTitleQuery;
use backend_types::media::{ItemUuid, MangaFormat, MediaAirDate, MediaChapterAmount, MediaDescription, MediaId, MediaSource, MediaStatus, MediaTitle, MediaTitles, MediaType, MediaVolumeAmount, SourceId};
use backend_types::media::data_source::{DataQueryType, DataSourceBaseUrl, DataSourceName, Media, DataSourceRequest, DataSourceResponse, MediaExternalRequest, MediaHttpClient, MediaInternalRequest, MediaInternalResponse, MediaRequest, MediaRequestId, MediaSourceId, NoInternalMedia, SupportedMediaTypes, SupportedQueryTypes};
use backend_types::schemas::storage::MangaBuilder;
use bevy_ecs::entity::Entity;
use bevy_ecs::error::BevyError;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::lifecycle::Add;
use bevy_ecs::observer::On;
use bevy_ecs::query::{AnyOf, With};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedule, ScheduleLabel, SystemSet};
use bevy_ecs::system::{Commands, In, IntoSystem, Populated, Query, Single};
use bevy_ecs::world::EntityRef;
use governor::Quota;
use itertools::Itertools;
use uuid::Uuid;

use self::mangabaka::MangaBakaClient;

// pub mod anilist;
// pub mod mal;
pub mod mangabaka;
// pub mod models;
// pub mod errors;
// pub mod components;
// pub mod hardcover;
// pub mod comicvine;
// pub mod tmdb;
// pud mod tvdb;
// pud mod musicbrainz;

pub fn dispatch_media_request(
    add: On<Add, MediaInternalRequest>,
    reqs: Query<(&MediaInternalRequest, &MediaTitleQuery), With<MediaInternalRequest>>,
    media: Query<(&ItemUuid, &Media, &MediaTitles)>,
    mut cmds: Commands,
) -> Result<(), BevyError> {
    let (req_info, query) = reqs.get(add.entity).unwrap();
    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

    // @todo: swap the `find` for a `filter` and `sort`
    // @todo: add a scoring threshold such that loose matches aren't hit
    match media.iter()
        .filter(|(_, m, t)|
            m.0 == req_info.media_type()
        )
        .sorted_by(|(_, _, a), (_, _, b)| Ord::cmp(
            &a.0.iter().filter_map(|t| matcher.fuzzy(&t.title, &query.0, false))
                .sorted_by(|a, b| Ord::cmp(&a.0, &b.0)).next().unwrap_or_default().0,
            &b.0.iter().filter_map(|t| matcher.fuzzy(&t.title, &query.0, false))
                .sorted_by(|a, b| Ord::cmp(&a.0, &b.0)).next().unwrap_or_default().0,
        ))
            .next()
    {
        Some((ItemUuid(id), _, _)) => {
            cmds.spawn((
                MediaInternalResponse::from_request(req_info.clone()), MediaId(*id),
            ));
            cmds.entity(add.entity).despawn();
        }
        None => {
            return Err(BevyError::new(bevy_ecs::error::Severity::Info, InternalMediaNotFound { entity: add.entity }));
        }
    }
    Ok(())
}
pub fn dispatch_data_source_request(
    add: On<Add, MediaExternalRequest>,
    reqs: Query<(&MediaExternalRequest, &MediaTitleQuery)>,
    media_clients: Populated<(Entity, &DataSourceName, &SupportedMediaTypes), With<DataSourceBaseUrl>>,
    mut cmds: Commands,
) {
    let (req_info, query) = reqs.get(add.entity).unwrap();

    for ent in media_clients.iter()
        .filter(|(_,n,s)|
            req_info.sources().map(|s| s.contains(&n.0)).unwrap_or(true)
                && s.0.contains(req_info.media_type())
        )
        .map(|(e,_,_)| e)
    {
        let mut req = cmds.spawn((
            ChildOf(ent),
            MediaRequestId(req_info.id()),
            DataSourceRequest::new(req_info.query_type()),
            query.clone(),
            req_info.media_type(),
        ));
    }
    cmds.entity(add.entity).despawn();
}

pub fn dispatch_next_request(
    add: On<Add, DataSourceResponse>,
    resps: Query<(
        &ChildOf, &MediaRequestId, &DataSourceResponse, &MediaSourceId, &MediaType,
    )>,
    sources: Populated<(Entity, &SupportedQueryTypes)>,
    mut cmds: Commands,
) {
    let (source, req_id, resp, media_id, media_type) = resps.get(add.entity).unwrap();
    let (_, SupportedQueryTypes(query_types)) = sources.get(source.0).unwrap();

    match resp.query_type() {
        DataQueryType::Search => if query_types.contains(DataQueryType::Info) {
            cmds.spawn((
                source.clone(), req_id.clone(),
                DataSourceRequest::new(DataQueryType::Info), media_id.clone(),
                *media_type,
            ));
            // for now, only despawn the search response if we'll acquire the media info
            cmds.entity(add.entity).despawn();
        }
        // we don't despawn the info request since its used to build the final media
        DataQueryType::Info => {
            // filter out the info + search requests since we've already hit those
            for query_ty in query_types.difference(DataQueryType::Info | DataQueryType::Search) {
                cmds.spawn((
                    source.clone(), req_id.clone(),
                    DataSourceRequest::new(query_ty), media_id.clone(),
                    *media_type,
                ));
            }
        }
        _ => {}
    }
}

pub fn manga_info_builder(
    media: Populated<
        (EntityRef, &MediaRequestId, &MediaType, &DataSourceResponse),
    >,
    clients: Populated<(Entity, &DataSourceName)>,
    mut cmds: Commands,
) {
    let id_groups = media.iter()
        .filter(|(_, _, t, r)| **t == MediaType::Manga && r.query_type() == DataQueryType::Info)
        .fold(BTreeMap::<MediaRequestId, Vec<EntityRef>>::new(), |mut acc, (ent, id, _, _)| {
            acc.entry(id.clone()).or_default().push(ent);
            acc
        });

    for (id, query_group) in id_groups {
        // @todo: add field checks here (have we already inserted the field into the builder?)
        let mut builder = MangaBuilder::default();
        builder.id(id.0);

        for query in query_group {
            if let Some(fmt) = query.get::<MangaFormat>() {
                builder.format(fmt.clone());
            }
            if let Some(ch) = query.get::<MediaChapterAmount>() {
                builder.chapters(Some(ch.0));
            }
            if let Some(vol) = query.get::<MediaVolumeAmount>() {
                builder.volumes(Some(vol.0));
            }
            if let Some(desc) = query.get::<MediaDescription>() {
                builder.description(Some(desc.0.clone()));
            }
            if let Some(air_date) = query.get::<MediaAirDate>() {
                if let Some(start_date) = &air_date.start {
                    builder.start_date(Some(start_date.clone()));
                }
                if let Some(end_date) = &air_date.end {
                    builder.end_date(Some(end_date.clone()));
                }
            }
            if let Some(source) = query.get::<MediaSource>() {
                builder.source(source.clone());
            }
            if let Some(status) = query.get::<MediaStatus>() {
                builder.status(status.clone());
            }
            if let Some(source_id) = query.get::<MediaSourceId>() {
                let source_id = source_id.0;
                let source_name = clients
                    .get(query.get::<ChildOf>().unwrap().parent()).unwrap().1.0.clone();
                builder.insert_source_id(SourceId { source: source_name, id: source_id });
            }
            if let Some(titles) = query.get::<MediaTitles>() {
                builder.insert_titles(titles.clone());
            }

            cmds.entity(query.id()).despawn();
        }

        let manga = builder.build().expect("could not build manga from info responses");
        let mut ent = cmds.spawn((
            ItemUuid(manga.id), Media(MediaType::Manga), manga.format,
            manga.source, manga.status, manga.titles, manga.source_ids,
            MediaAirDate { start: manga.start_date, end: manga.end_date },
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

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataQueryingSystems {
    Search,
    Info,
    Relations,
}

pub struct DataSourcePlugin;

impl bevy_app::Plugin for DataSourcePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // @todo: convert the external queries into either observers or message passers (as in repsonses are passed to the next request system in the chain)
        // do i want a sub world dedicated to data source querying?
        app.world_mut()
            .spawn(MediaHttpClient::new(
                MangaBakaClient::data_source(), Quota::per_minute(30u32.try_into().unwrap())
            ));
        // @todo: better handle internal media errors (ex. no internal media found)
        app.add_observer(dispatch_media_request.pipe(handle_internal_media_error))
            .add_observer(dispatch_data_source_request)
            .add_observer(dispatch_next_request);
        // how should i orchestrate the schedule?
        // do i wanna messaging system between the dispatchers and the query systems?
        app.add_systems(bevy_app::FixedUpdate, (
            mangabaka::search_request,
        ).in_set(DataQueryingSystems::Search));
        app.add_systems(bevy_app::FixedUpdate, (
            mangabaka::info_request
        ).in_set(DataQueryingSystems::Info));
        app.configure_sets(bevy_app::FixedUpdate, (
            DataQueryingSystems::Search,
            DataQueryingSystems::Info,
            DataQueryingSystems::Relations,
        ).chain());
        // data aggregation systems
        app.add_systems(bevy_app::FixedPostUpdate, (
            manga_info_builder,
        ));
    }
}

#[derive(Debug)]
struct InternalMediaNotFound {
    entity: Entity,
}
impl std::fmt::Display for InternalMediaNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entity not found in the internal ECS system")
    }
}
impl std::error::Error for InternalMediaNotFound {}

fn handle_internal_media_error(
    In(res): In<Result<(), BevyError>>,
    reqs: Query<(&MediaInternalRequest, &MediaTitleQuery)>,
    // @todo: move the cli types into `backend_types` to pull this in
    cli_query: Option<Single<&CliInternalQuery>>,
    mut cmds: Commands,
) {
    if let Err(e) = res {
        let ent = e.downcast_ref::<InternalMediaNotFound>().unwrap().entity;
        let (req, query) = reqs
            .get(ent)
            .unwrap();
        cmds.entity(ent).despawn();

        // there must be a better way to match against this (possibly in `cli_internal_query()`)
        if let Some(query) = cli_query
            && query.id == req.id()
        {
            // @todo: vvv load the query into here (+ bikeshed name)
            cmds.trigger(NoInternalMedia);
        } else {
            // fallback to pinging external sources
            cmds.spawn((
                MediaExternalRequest::from_internal_req(req.clone()),
                query.clone()
            ));
        }
    }
}
