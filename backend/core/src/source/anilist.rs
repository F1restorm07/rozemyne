use std::pin::Pin;

use backend_schemas::anilist::{
    self, AnimeInfoQuery, MangaInfoQuery, MediaFormat, MediaRelationsQuery, MediaSearchQuery, MediaSource, MediaStatus, MediaTitle, SearchRequest, SeriesRequest
};
use backend_schemas::db::{AnimeFormat, FuzzyDate, MangaFormat, MediaSeason, Season, Source, Status};
use cynic::{GraphQlResponse, QueryBuilder};
use reqwest::{Body, Client, ClientBuilder, Method, Request, Url, header};
use tower::{Service, ServiceExt};
use tower::util::BoxService;

use super::{Anime, ClientError, EpisodicMedia, Manga, RequestData, ResponseData, SearchResponse, SourceIds, Title};

pub struct AnilistClient {
    client: Client,
}

impl AnilistClient {
    pub fn new(config: ClientBuilder) -> Self {
        Self {
            client: config.build().unwrap(),
        }
    }
}

impl Service<super::Request> for AnilistClient {
    type Response = super::Response;
    type Error = ClientError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: super::Request) -> Self::Future {
        if let super::Request::Http(req) = req {
            let res = self.client.execute(*req);
            Box::pin(async { Ok(super::Response::Http(
                        res.await.map_err(ClientError::Http)?
            )) })
        } else { Box::pin(async { Err(ClientError::InvalidRequestType) }) }
    }
    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

pub fn search(
    req: RequestData,
    backend: &mut BoxService<super::Request, super::Response, ClientError>,
    base: Url
) -> Pin<Box<dyn Future<Output = Result<Option<ResponseData>, ClientError>> + '_>> {
    Box::pin(async move {
        match req {
            RequestData::Search(super::SearchRequest { series, ty }) => {
                let ty = match ty {
                    super::MediaType::Anime => anilist::MediaType::Anime,
                    super::MediaType::Manga => anilist::MediaType::Manga,
                    _ => return Err(ClientError::InvalidRequestType)
                };
                let data = MediaSearchQuery::build(SearchRequest { search: Some(series), ty: Some(ty) });
                let mut req = Request::new(Method::POST, base);
                let mut headers = header::HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
                *req.body_mut() = Some(serde_json::to_string(&data).unwrap().into());
                *req.headers_mut() = headers;

                let backend = backend.ready().await?;
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<GraphQlResponse<MediaSearchQuery>>().await?;
                let media = res.data.as_ref().unwrap()
                    .page.as_ref().unwrap()
                    .media.as_ref().unwrap().first();
                match media {
                    Some(media) => {
                        let media = media.as_ref().unwrap();
                        let title = media.title.as_ref().unwrap_or(&MediaTitle { romaji: None, english: None, native: None });

                        Ok(Some(ResponseData::Search(SearchResponse {
                            id: media.id as u32,
                            native_title: title.romaji.clone(),
                            english_title: title.english.clone(),
                        })))
                    }
                    None => Ok(None)
                }
            }
            _ => Err(ClientError::InvalidRequestType)
        }
    })
}

pub fn info(
    req: RequestData,
    backend: &mut BoxService<super::Request, super::Response, ClientError>,
    base: Url
) -> Pin<Box<dyn Future<Output = Result<Option<ResponseData>, ClientError>> + '_>> {
    Box::pin(async move {
        match req {
            RequestData::Info(super::InfoRequest { id, ty: super::MediaType::Anime }) => {
                let data = AnimeInfoQuery::build(SeriesRequest { id: id as i32 });
                let mut req = Request::new(Method::POST, base);
                let mut headers = header::HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
                *req.body_mut() = Some(Body::wrap(serde_json::to_string(&data).unwrap()));
                *req.headers_mut() = headers;

                let backend = backend.ready().await?;
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<GraphQlResponse<AnimeInfoQuery>>().await?;
                let media = res.data.as_ref().unwrap()
                    .media.as_ref().unwrap();

                let source = match media.source.as_ref().unwrap_or(&MediaSource::Other) {
                    MediaSource::Anime => Source::Anime,
                    MediaSource::Manga => Source::Manga,
                    MediaSource::LightNovel => Source::LightNovel,
                    MediaSource::VisualNovel => Source::VisualNovel,
                    MediaSource::WebNovel => Source::WebNovel,
                    MediaSource::Novel => Source::Novel,
                    MediaSource::Comic => Source::Comic,
                    MediaSource::Doujinshi => Source::Doujinshi,
                    MediaSource::LiveAction => Source::LiveAction,
                    MediaSource::MultimediaProject => Source::MultimediaProject,
                    MediaSource::PictureBook => Source::PictureBook,
                    MediaSource::Game => Source::Game,
                    MediaSource::VideoGame => Source::VideoGame,
                    MediaSource::Original => Source::Original,
                    MediaSource::Other => Source::Other,
                };
                let status = match media.status.as_ref().unwrap_or(&MediaStatus::NotYetReleased) {
                    MediaStatus::NotYetReleased => Status::Upcoming,
                    MediaStatus::Finished => Status::Finished,
                    MediaStatus::Releasing => Status::Releasing,
                    MediaStatus::Cancelled => Status::Cancelled,
                    MediaStatus::Hiatus => Status::Hiatus,
                };
                let format = match media.format.as_ref().unwrap_or(&MediaFormat::Tv) {
                    MediaFormat::Tv => AnimeFormat::TVShow,
                    MediaFormat::TvShort => AnimeFormat::TVShort,
                    MediaFormat::Movie => AnimeFormat::Movie,
                    MediaFormat::Music => AnimeFormat::Other,
                    MediaFormat::Ona => AnimeFormat::Ona,
                    MediaFormat::Ova => AnimeFormat::Ova,
                    MediaFormat::Special => AnimeFormat::Special,
                    _ => unreachable!("this media is an anime")
                };

                let ep_media = EpisodicMedia::new(
                    media.description.clone(),
                    media.start_date.as_ref().map(|d| FuzzyDate {
                        year: d.year.map(|y| y as i16),
                        month: d.month.map(|m| m as u8),
                        day: d.day.map(|d| d as u8),

                    }),
                    media.end_date.as_ref().map(|d| FuzzyDate {
                        year: d.year.map(|y| y as i16),
                        month: d.month.map(|m| m as u8),
                        day: d.day.map(|d| d as u8),

                    }),
                    source,
                    status,
                    media.title.as_ref().map(|t| Title {
                        romaji: t.romaji.clone(),
                        english: t.english.clone(),
                        native: t.native.clone(),
                        synonyms: media.synonyms.as_ref()
                            .map(|s| s.iter().filter_map(|t| t.clone()).collect())
                            .unwrap_or(vec![])
                    }).unwrap_or(Title { romaji: None, english: None, native: None, synonyms: vec![] })
                );

                Ok(Some(ResponseData::Anime(Anime {
                    format,
                    air_season: media.season.as_ref().map(|s| MediaSeason {
                        year: media.season_year.map(|y| y as i16).unwrap_or_default(),
                        season: match s {
                            anilist::MediaSeason::Winter => Season::Winter,
                            anilist::MediaSeason::Spring => Season::Spring,
                            anilist::MediaSeason::Summer => Season::Summer,
                            anilist::MediaSeason::Fall => Season::Fall,
                        }
                    }),
                    num_episodes: media.episodes,
                    episode_length: media.duration,
                    episodes: vec![],
                    ep_media,
                    source_ids: SourceIds {
                        ids: vec![("AniList".to_string(), media.id as u32)]
                    }
                })))

            }
            RequestData::Info(super::InfoRequest { id, ty: super::MediaType::Manga }) => {
                let data = MangaInfoQuery::build(SeriesRequest { id: id as i32 });
                let mut req = Request::new(Method::POST, base);
                let mut headers = header::HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
                *req.body_mut() = Some(Body::wrap(serde_json::to_string(&data).unwrap()));
                *req.headers_mut() = headers;

                let backend = backend.ready().await?;
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<GraphQlResponse<MangaInfoQuery>>().await?;
                let media = res.data.as_ref().unwrap()
                    .media.as_ref().unwrap();

                let source = match media.source.as_ref().unwrap_or(&MediaSource::Other) {
                    MediaSource::Anime => Source::Anime,
                    MediaSource::Manga => Source::Manga,
                    MediaSource::LightNovel => Source::LightNovel,
                    MediaSource::VisualNovel => Source::VisualNovel,
                    MediaSource::WebNovel => Source::WebNovel,
                    MediaSource::Novel => Source::Novel,
                    MediaSource::Comic => Source::Comic,
                    MediaSource::Doujinshi => Source::Doujinshi,
                    MediaSource::LiveAction => Source::LiveAction,
                    MediaSource::MultimediaProject => Source::MultimediaProject,
                    MediaSource::PictureBook => Source::PictureBook,
                    MediaSource::Game => Source::Game,
                    MediaSource::VideoGame => Source::VideoGame,
                    MediaSource::Original => Source::Original,
                    MediaSource::Other => Source::Other,
                };
                let status = match media.status.as_ref().unwrap_or(&MediaStatus::NotYetReleased) {
                    MediaStatus::NotYetReleased => Status::Upcoming,
                    MediaStatus::Finished => Status::Finished,
                    MediaStatus::Releasing => Status::Releasing,
                    MediaStatus::Cancelled => Status::Cancelled,
                    MediaStatus::Hiatus => Status::Hiatus,
                };
                let format = match media.format.as_ref().unwrap_or(&MediaFormat::Manga) {
                    MediaFormat::Manga => MangaFormat::Manga,
                    MediaFormat::Novel => MangaFormat::LightNovel,
                    MediaFormat::OneShot => MangaFormat::Other,
                    _ => unreachable!("this media is a manga")
                };

                let ep_media = EpisodicMedia::new(
                    media.description.clone(),
                    media.start_date.as_ref().map(|d| FuzzyDate {
                        year: d.year.map(|y| y as i16),
                        month: d.month.map(|m| m as u8),
                        day: d.day.map(|d| d as u8),

                    }),
                    media.end_date.as_ref().map(|d| FuzzyDate {
                        year: d.year.map(|y| y as i16),
                        month: d.month.map(|m| m as u8),
                        day: d.day.map(|d| d as u8),

                    }),
                    source,
                    status,
                    media.title.as_ref().map(|t| Title {
                        romaji: t.romaji.clone(),
                        english: t.english.clone(),
                        native: t.native.clone(),
                        synonyms: media.synonyms.as_ref()
                            .map(|s| s.iter().filter_map(|t| t.clone()).collect())
                            .unwrap_or(vec![])
                    }).unwrap_or(Title { romaji: None, english: None, native: None, synonyms: vec![] })
                );

                Ok(Some(ResponseData::Manga(Manga {
                    format,
                    chapters: media.chapters.map(|c| c as u32),
                    volumes: media.volumes.map(|v| v as u32),
                    ep_media,
                    source_ids: SourceIds {
                        ids: vec![("AniList".to_string(), media.id as u32)]
                    }
                })))
            }
            _ => Err(ClientError::InvalidRequestType)
        }
    })
}

pub fn relations(
    req: RequestData,
    backend: &mut BoxService<super::Request, super::Response, ClientError>,
    base: Url
) -> Pin<Box<dyn Future<Output = Result<ResponseData, ClientError>> + '_>> {
    Box::pin(async move {
        if let RequestData::Relation(info) = req {
            let data = MediaRelationsQuery::build(SeriesRequest { id: info.id as i32 });
            let mut req = Request::new(Method::POST, base);
            let mut headers = header::HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
            *req.body_mut() = Some(Body::wrap(serde_json::to_string(&data).unwrap()));
            *req.headers_mut() = headers;

                let backend = backend.ready().await?;
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<GraphQlResponse<MediaRelationsQuery>>().await?;
                // ignoring characters and other relations, as they have their own requests
                let _relations = res.data.as_ref().unwrap()
                    .media.as_ref().unwrap();

            Ok(ResponseData::Relations(super::RelationsResponse {
                relations: vec![],
                characters: vec![],
                staff: vec![],
                studios: vec![],
            }))

        } else { Err(ClientError::InvalidRequestType) }
    })
}
