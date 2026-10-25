use backend_types::media::query::MediaTitleQuery;
use backend_types::media::{ContentRating, FuzzyDate, MangaFormat, MediaAirDate, MediaDescription, MediaGenres, MediaRelation, MediaRelations, MediaSource, MediaStaff, MediaStaffMemberBuilder, MediaStatus, MediaTagBuilder, MediaTags, MediaTitle, MediaTitles, MediaType, SourceId};
use backend_types::media::data_source::{DataQueryType, DataSource, DataSourceBaseUrl, DataSourceName, HttpClient, DataSourceRequest, DataSourceResponse, MediaRequestId, MediaSourceId, RateLimiter};
use backend_types::schemas::mangabaka::{MangaBakaResponse, SearchRequest, Series};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Query, Single};
use bevy_ecs::world::EntityRef;
use ureq::http::Request;

#[derive(Component)]
pub struct MangaBakaClient;

impl MangaBakaClient {
    pub fn data_source() -> DataSource<MangaBakaClient> {
        DataSource::new(
            MangaBakaClient,
            "MangaBaka",
            "https://api.mangabaka.org/",
            enumset::EnumSet::only(MediaType::Manga),
            DataQueryType::Search | DataQueryType::Info,
        )
    }
}

pub fn search_request(
    reqs: Query<(Entity, &MediaRequestId, &ChildOf, &DataSourceRequest, &MediaTitleQuery, &MediaType)>,
    // `Single` as there should only ever be one instance of a media given client
    mangabaka_client: Single<(Entity, &DataSourceBaseUrl, &HttpClient, &RateLimiter), With<MangaBakaClient>>,
    mut cmds: Commands
) {
    let (source_ent, DataSourceBaseUrl(base_url), client, limiter) = mangabaka_client.into_inner();

    let requests = reqs.iter()
        .filter(|(_, _, parent, req, _, media_type)|
        parent.parent() == source_ent && **media_type == MediaType::Manga && req.query_type() == DataQueryType::Search
    ).map(|(ent, media_id, _, req, query, media_type)| (ent, media_id, req, query, media_type));

    for (req_ent, media_id, req, MediaTitleQuery(query), media_type) in requests {
        let search_req = SearchRequest {
            titles: Some(query.clone()),
            ..Default::default()
        };

        let mut url = base_url.join("v1/series/search").unwrap();
        url.set_query(Some(&serde_urlencoded::to_string(search_req).unwrap()));

        futures_lite::future::block_on(async { limiter.0.until_ready().await; });

        let response = client.0.get(url.as_str()).call().unwrap();
        let mut body = response.into_body();
        let res = &body
            .read_json::<MangaBakaResponse<Vec<Series>>>().unwrap().data;

        if let Some(res) = res.first() {
            cmds.spawn((
                ChildOf(source_ent), media_id.clone(), *media_type,
                DataSourceResponse::from_media_req(req), MediaSourceId(res.id.0 as u32),
            ));
        }
        // clear the request from the `World`
        cmds.entity(req_ent).despawn();
    }
}

pub fn info_request(
    reqs: Query<(Entity, &MediaRequestId, &ChildOf, &DataSourceRequest, &MediaSourceId, &MediaType)>,
    mangabaka_client: Single<(Entity, &DataSourceName, &DataSourceBaseUrl, &HttpClient, &RateLimiter)>,
    mut cmds: Commands
) {
    let (source_ent, DataSourceName(source_name), DataSourceBaseUrl(base_url), client, limiter) = mangabaka_client.into_inner();

    let requests = reqs.iter().filter(|(_, _, parent, req, _, media_type)|
        parent.parent() == source_ent && **media_type == MediaType::Manga && req.query_type() == DataQueryType::Info
    ).map(|(ent, media_id, _, req, query, media_type)| (ent, media_id, req, query, media_type));

    for (req_ent, req_id, req, MediaSourceId(media_id), media_type) in requests {
        let url = base_url.join(&format!("v1/series/{media_id}")).unwrap();

        futures_lite::future::block_on(async { limiter.0.until_ready().await; });

        let response = client.0.get(url.as_str()).call().unwrap();
        let mut body = response.into_body();
        let res = &body
            .read_json::<MangaBakaResponse<Series>>().unwrap().data;

        let mut mangabaka_res = cmds.spawn((
            ChildOf(source_ent), req_id.clone(), *media_type, DataSourceResponse::from_media_req(req),
            MediaSourceId(res.id.0 as u32), MangaFormat::from(res.r#type.clone()), MediaStatus::from(res.status.clone()),
            ContentRating::from(res.content_rating.clone()), MediaSource::Original,
        ));

        // decouple this from the main info entity, and check to see if a staff member with this name exists
        // add a relationship between the info entity and the staff member
        // if the staff member exists in the ECS, just add the relationship
        // else, add the entity with just the name + relationship (no source id as MangaBaka doesn't carry ids for staff members)
        match (res.authors.clone(), res.artists.clone()) {
            (Some(authors), Some(artists)) => {
                let authors = authors.iter().chain(artists.iter()).map(|a|
                    MediaStaffMemberBuilder::default()
                        .name(a.clone())
                        // .source_id(SourceId { source: source_name.clone(), id: None })
                        .build().expect("could not build media staff member")
                ).collect();
                mangabaka_res.insert(MediaStaff(authors));
            }
            (Some(authors), None) => {
                let authors = authors.iter().map(|a|
                    MediaStaffMemberBuilder::default()
                    .name(a.clone())
                    // .source_id(SourceId { source: source_name.clone(), id: None })
                    .build().expect("could not build media staff member")
                ).collect();
                mangabaka_res.insert(MediaStaff(authors));
            }
            (None, Some(artists)) => {
                let authors = artists.iter().map(|a|
                    MediaStaffMemberBuilder::default()
                        .name(a.clone())
                        // .source_id(SourceId { source: source_name.clone(), id: None })
                        .build().expect("could not build media staff member")
                ).collect();
                mangabaka_res.insert(MediaStaff(authors));
            }
            (None, None) => {}
        }
        if let Some(desc) = &res.description {
            mangabaka_res.insert(MediaDescription(desc.clone()));
        }
        if let Some(publication) = &res.published
            && (publication.start_date.is_some() || publication.end_date.is_some())
        {
            mangabaka_res.insert(MediaAirDate {
                start: publication.start_date.as_ref().map(|d| {
                    let date = d.parse::<jiff::civil::Date>().unwrap();
                    FuzzyDate {
                        year: Some(date.year()),
                        month: Some(date.month() as u8),
                        day: Some(date.day() as u8),
                    }
                }),
                end: publication.end_date.as_ref().map(|d| {
                    let date = d.parse::<jiff::civil::Date>().unwrap();
                    FuzzyDate {
                        year: Some(date.year()),
                        month: Some(date.month() as u8),
                        day: Some(date.day() as u8),
                    }
                }),
            });
        }
        if let Some(tags) = &res.tags_v2 {
            // this is the *only* way to collect the genre names on MangaBaka (`genres_v2` is always null)
            if tags.iter().any(|t| t.is_genre) {
                let genres = tags.iter().filter(|t| t.is_genre)
                    .map(|t| t.name.clone())
                    .collect();
                mangabaka_res.entry::<MediaGenres>().or_insert(MediaGenres(genres));
            }

            let tags = tags.iter().map(|t| {
                MediaTagBuilder::default()
                    .name(t.name.clone())
                    .description(t.description.clone())
                    .category(Some(t.name_path.clone()))
                    .is_spoiler(t.is_spoiler)
                    .content_rating(backend_types::media::ContentRating::from(t.content_rating.clone()))
                    .build().expect("could not build media tag")
            }).collect();
            mangabaka_res.insert(MediaTags(tags));
        }
        if let Some(titles) = &res.titles {
            let titles = titles.iter().map(|t| {
                let mut traits = t.traits.iter()
                    .fold(enumset::EnumSet::new(), |e, t| e | backend_types::media::TitleTrait::from(t.clone()));
                if t.is_primary {
                    traits |= backend_types::media::TitleTrait::Primary;
                }
                MediaTitle {
                    title: t.title.clone(),
                    language: Some(t.language.clone()),
                    traits,
                }
            }).collect();
            mangabaka_res.insert(MediaTitles(titles));
        }
        // @todo: support relationship requests (MangaBaka supports them)
        if !res.relationships_v2.is_empty() {
            let relationships = res.relationships_v2.iter().map(|r| {
                MediaRelation {
                    target_id: SourceId { source: source_name.clone(), id: r.to_series_id },
                    rel_type: backend_types::media::RelationType::from(r.relation_type.clone()),
                    target_type: backend_types::media::MediaType::Manga,
                }
            }).collect::<Vec<_>>();

            // @todo: find a way to introspect the relation (probably by sending a new request) to find the format type
            // if relationships.iter().any(|r| r.rel_type == super::models::RelationType::Source) {
            // }
            // spawn a separate entity with a different response type to separate the relations
            // from the rest of the request (they get processed separately)
            // cmds.spawn((
            //     ChildOf(source_ent), req_id.clone(), media_type.clone(),
            //     MediaDataResponse::<MediaRelationsQuery>::new(req.req_id()),
            //     MediaRelations(relationships),
            // ));
        }

        // clear the request from the `World`
        cmds.entity(req_ent).despawn();
    }
}
