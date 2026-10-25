use std::pin::Pin;

use reqwest::{Client, ClientBuilder, Method, Request, Url};
use reqwest::header::{self, HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use tower::Service;
use tower::util::BoxService;

use super::{ClientError, RequestData, ResponseData, SearchResponse};

pub struct MalClient {
    client: Client,
    api_key: SecretString,
}

impl MalClient {
    pub fn new(config: ClientBuilder, api_key: SecretString) -> Self {
        let mut map = HeaderMap::new();
        map.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
        Self {
            client: config.default_headers(map).build().unwrap(),
            api_key,
        }
    }

    // pub fn anime(&self, req: SeriesRequest) -> Result<AnimeInfo, ureq::Error> {
    //     let query = "fields=id,title,synopsis,alternative_titles,start_date,end_date,nsfw,media_type,status,source,num_episodes,average_episode_duration,start_season,main_picture";
    //     let uri = Uri::builder()
    //         .scheme("https")
    //         .authority("api.myanimelist.net")
    //         .path_and_query(format!("/v2/anime/{}?{query}", req.id))
    //         .build()
    //         .unwrap();
    //
    //     let req = self
    //         .agent
    //         .get(uri)
    //         .header(
    //             "Authorization",
    //             format!("Bearer {}", self.api_key.expose_secret()),
    //         )
    //         .call()?;
    //
    //     req.into_body().read_json::<AnimeInfo>()
    // }
    // pub fn anime_relations(&self, req: SeriesRequest) -> Result<AnimeRelations, ureq::Error> {
    //     let query =
    //         "fields=studios,pictures,genres,related_anime{id,title},related_manga{id,title}";
    //     let uri = Uri::builder()
    //         .scheme("https")
    //         .authority("api.myanimelist.net")
    //         .path_and_query(format!("/v2/anime/{}?{query}", req.id))
    //         .build()
    //         .unwrap();
    //
    //     let req = self
    //         .agent
    //         .get(uri)
    //         .header(
    //             "Authorization",
    //             format!("Bearer {}", self.api_key.expose_secret()),
    //         )
    //         .call()?;
    //
    //     req.into_body().read_json::<AnimeRelations>()
    // }
    //
    // pub fn manga(&self, req: SeriesRequest) -> Result<MangaInfo, ureq::Error> {
    //     let query = "fields=id,title,synopsis,alternative_titles,start_date,end_date,nsfw,media_type,status,source,num_episodes,average_episode_duration,start_season,main_picture";
    //     let uri = Uri::builder()
    //         .scheme("https")
    //         .authority("api.myanimelist.net")
    //         .path_and_query(format!("/v2/manga/{}?{query}", req.id))
    //         .build()
    //         .unwrap();
    //
    //     let req = self
    //         .agent
    //         .get(uri)
    //         .header(
    //             "Authorization",
    //             format!("Bearer {}", self.api_key.expose_secret()),
    //         )
    //         .call()?;
    //
    //     req.into_body().read_json::<MangaInfo>()
    // }
    // pub fn manga_relations(&self, req: SeriesRequest) -> Result<MangaRelations, ureq::Error> {
    //     let query =
    //         "fields=authors,pictures,genres,related_anime{id,title},related_manga{id,title}";
    //     let uri = Uri::builder()
    //         .scheme("https")
    //         .authority("api.myanimelist.net")
    //         .path_and_query(format!("/v2/manga/{}?{query}", req.id))
    //         .build()
    //         .unwrap();
    //
    //     let req = self
    //         .agent
    //         .get(uri)
    //         .header(
    //             "Authorization",
    //             format!("Bearer {}", self.api_key.expose_secret()),
    //         )
    //         .call()?;
    //
    //     req.into_body().read_json::<MangaRelations>()
    // }
}

impl Service<super::Request> for MalClient {
    type Response = super::Response;
    type Error = ClientError;
    // `Send` bound here is required for `BoxService`
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: super::Request) -> Self::Future {
        if let super::Request::Http(mut req) = req {
            let headers = req.headers_mut();
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", self.api_key.expose_secret())).unwrap()
            );
            let res = self.client.execute(*req);
            Box::pin(async { Ok(super::Response::Http(
                res.await.map_err(ClientError::Http)?
            ))})
        } else { Box::pin(async { Err(ClientError::InvalidRequestType) }) }
    }

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

pub fn search(
    req: RequestData,
    backend: &mut BoxService<super::Request, super::Response, ClientError>,
    mut base: Url
) -> Pin<Box<dyn Future<Output = exn::Result<ResponseData, ClientError>> + '_>> {
    Box::pin(async move {
        match req {
            RequestData::Search(super::SearchRequest { series, ty: super::MediaType::Anime }) => {
                base.set_path("/v2/anime");
                base.set_query(Some(&format!("q={series},fields=id,title")));

                let req = Request::new(Method::GET, base);
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<MediaSearch>().await?;
                let title = &res.data[0].node;

                exn::Ok(ResponseData::Search(SearchResponse {
                    id: title.id as u32,
                    native_title: Some(title.title.clone()),
                    english_title: None,
                }))
            }
            RequestData::Search(super::SearchRequest { series, ty: super::MediaType::Manga }) => {
                base.set_path("/v2/manga");
                base.set_query(Some(&format!("q={series},fields=id,title")));

                let req = Request::new(Method::GET, base);
                let res = backend.call(super::Request::Http(Box::new(req))).await?;
                let res = res.json::<MediaSearch>().await?;
                let title = &res.data[0].node;

                exn::Ok(ResponseData::Search(SearchResponse {
                    id: title.id as u32,
                    native_title: Some(title.title.clone()),
                    english_title: None,
                }))
            }
            _ => { Err(ClientError::InvalidRequestType.into()) }
        }
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct AnimeInfo {
    id: i32,
    title: String,
    synopsis: Option<String>,
    alternative_titles: Option<MediaTitle>,
    start_date: Option<String>,
    end_date: Option<String>,
    nsfw: Option<Nsfw>,
    media_type: AnimeType,
    status: Status,
    source: Option<Source>,
    num_episodes: i32,                     // 0 if unknown
    average_episode_duration: Option<i32>, // length in seconds
    start_season: Option<MediaSeason>,
    main_picture: Option<MediaPicture>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaInfo {
    id: i32,
    title: String,
    synopsis: Option<String>,
    alternative_titles: Option<MediaTitle>,
    start_date: Option<String>,
    end_date: Option<String>,
    nsfw: Option<Nsfw>,
    media_type: MangaType,
    status: Status,
    num_volumes: i32,  // 0 if unknown
    num_chapters: i32, // 0 if unknown
    main_picture: Option<MediaPicture>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaTitle {
    en: Option<String>,
    jp: Option<String>,
    synonyms: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
pub enum Nsfw {
    #[serde(rename = "white")]
    White,
    #[serde(rename = "gray")]
    Gray,
    #[serde(rename = "black")]
    Black,
}

#[derive(Debug, serde::Deserialize)]
pub enum AnimeType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "tv")]
    Tv,
    #[serde(rename = "ova")]
    Ova,
    #[serde(rename = "ona")]
    Ona,
    #[serde(rename = "movie")]
    Movie,
    #[serde(rename = "special")]
    Special,
    #[serde(rename = "music")]
    Music,
}

#[derive(Debug, serde::Deserialize)]
pub enum MangaType {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "manga")]
    Manga,
    #[serde(rename = "novel")]
    Novel,
    #[serde(rename = "one_shot")]
    Oneshot,
    #[serde(rename = "doujinshi")]
    Doujinshi,
    #[serde(rename = "manhwa")]
    Manhwa,
    #[serde(rename = "manhua")]
    Manhua,
    #[serde(rename = "oel")]
    OriginalEnglish,
}

#[derive(Debug, serde::Deserialize)]
pub enum Status {
    #[serde(rename = "finished_airing")]
    Finished,
    #[serde(rename = "currently_airing")]
    Releasing,
    #[serde(rename = "not_yet_aired")]
    Upcoming,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaSeason {
    year: i32,
    season: Season,
}

#[derive(Debug, serde::Deserialize)]
pub enum Season {
    #[serde(rename = "winter")]
    Winter,
    #[serde(rename = "spring")]
    Spring,
    #[serde(rename = "summer")]
    Summer,
    #[serde(rename = "fall")]
    Fall,
}

#[derive(Debug, serde::Deserialize)]
pub enum Source {
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "manga")]
    Manga,
    #[serde(rename = "4_koma_manga")]
    FourKomaManga,
    #[serde(rename = "web_manga")]
    WebManga,
    #[serde(rename = "digital_manga")]
    DigitalManga,
    #[serde(rename = "novel")]
    Novel,
    #[serde(rename = "light_novel")]
    LightNovel,
    #[serde(rename = "visual_novel")]
    VisualNovel,
    #[serde(rename = "game")]
    Game,
    #[serde(rename = "card_game")]
    CardGame,
    #[serde(rename = "book")]
    Book,
    #[serde(rename = "picture_book")]
    PictureBook,
    #[serde(rename = "radio")]
    Radio,
    #[serde(rename = "music")]
    Music,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaPicture {
    large: Option<String>,
    medium: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaSearch {
    data: Vec<MediaSearchNode>,
    paging: PageInfo,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaSearchNode {
    node: MediaSearchItem,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaSearchItem {
    id: i32,
    title: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct PageInfo {
    /// url for previous search page
    previous: String,
    /// url for next search page
    next: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AnimeRelations {
    studios: Vec<Studio>,
    related_anime: Vec<MediaEdge>,
    related_manga: Vec<MediaEdge>,
    pictures: Vec<MediaPicture>,
    genres: Vec<Genre>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaRelations {
    authors: Vec<AuthorEdge>,
    related_anime: Vec<MediaEdge>,
    related_manga: Vec<MediaEdge>,
    pictures: Vec<MediaPicture>,
    genres: Vec<Genre>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Genre {
    id: i32,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Studio {
    id: i32,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaEdge {
    node: MediaNode,
    relation_type: Relation,
}

#[derive(Debug, serde::Deserialize)]
pub struct MediaNode {
    id: i32,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
pub enum Relation {
    #[serde(rename = "sequel")]
    Sequel,
    #[serde(rename = "prequel")]
    Prequel,
    #[serde(rename = "alternative_setting")]
    AlternativeSetting,
    #[serde(rename = "alternative_version")]
    AlternativeVersion,
    #[serde(rename = "side_story")]
    SideStory,
    #[serde(rename = "parent_story")]
    Parent,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "full_story")]
    Full,
}

#[derive(Debug, serde::Deserialize)]
pub struct AuthorEdge {
    node: Person,
    role: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Person {
    id: i32,
    first_name: String,
    last_name: String,
}
