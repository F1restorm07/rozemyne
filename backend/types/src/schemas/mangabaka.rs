use crate::media::{ContentRating, MangaFormat, MediaStatus};

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaResponse<D> {
    pub status: u32,
    #[serde(skip)]
    _pagination: (),
    #[serde(skip)]
    _excluded_count: (),
    pub data: D,
}

#[derive(Debug, serde::Serialize, Default, Clone)]
pub struct SearchRequest {
    #[serde(rename(serialize = "q"))]
    pub titles: Option<String>,
    pub r#type: Option<Vec<MangaBakaSeriesType>>,
    pub type_not: Option<Vec<MangaBakaSeriesType>>,
    pub status: Option<Vec<MangaBakaStatus>>,
    pub status_not: Option<Vec<MangaBakaStatus>>,
    pub content_rating: Option<Vec<MangaBakaRating>>,
    pub not_content_rating: Option<Vec<MangaBakaRating>>,
    pub genre: Option<Vec<String>>, // the genre name (case insensitive)
    pub genre_not: Option<Vec<u32>>,
    pub tag: Option<Vec<u32>>, // the tag id (how can i find this???)
    pub tag_not: Option<Vec<u32>>,
    pub tag_mode: Option<String>, // either `and` or `or` (too niche for an enum right now)
    pub publisher: Option<String>,
    pub published_start_date_lower: Option<String>,
    pub published_start_date_upper: Option<String>,
    pub published_end_date_lower: Option<String>,
    pub published_end_date_upper: Option<String>,
    // @todo: finish implementing (see MangaBaka API explorer)
}

#[derive(Debug, serde::Deserialize)]
pub struct Series {
    pub anime: Option<MangaBakaAnime>,
    pub artists: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub content_rating: MangaBakaRating,
    pub description: Option<String>,
    pub final_volume: Option<String>,
    pub has_anime: bool,
    pub id: MangaBakaId,
    pub is_licensed: bool,
    pub last_updated_at: Option<String>,
    pub merged_with: Option<MangaBakaId>,
    /// Deprecated
    #[serde(skip)]
    _native_title: Option<String>,
    pub publishers: Option<Vec<MangaBakaPublisher>>,
    pub rating: Option<f32>,
    // @todo: update to v2 spec
    #[serde(skip)]
    /// Deprecated
    pub _relationships: (),
    pub relationships_v2: Vec<SeriesRelationship>,
    /// Deprecated
    #[serde(skip)]
    _romanized_title: Option<String>,
    /// Deprecated
    #[serde(skip)]
    _secondary_titles: (),
    pub source: MangaBakaSource,
    pub state: MangaBakaState,
    pub status: MangaBakaStatus,
    /// Deprecated
    #[serde(skip)]
    _tags: Vec<String>,
    /// Deprecated
    #[serde(skip)]
    _title: String,
    pub total_chapters: Option<String>,
    pub r#type: MangaBakaSeriesType,
    /// Deprecated
    #[serde(skip)]
    _year: Option<i32>,
    /// Deprecated, Optional
    #[serde(skip)]
    _genres: Option<Vec<String>>,
    /// Optional
    pub genres_v2: Option<Vec<MangaBakaTag>>,
    /// Optional
    pub links: Option<Vec<String>>,
    /// Optional
    pub published: Option<MangaBakaPublication>,
    /// Optional
    pub tags_v2: Option<Vec<MangaBakaTag>>,
    /// Optional
    pub titles: Option<Vec<MangaBakaTitle>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaId(pub i32);

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaAnime {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MangaBakaRating {
    Safe,
    Suggestive,
    Erotica,
    Pornographic,
}

impl From<MangaBakaRating> for ContentRating {
    fn from(value: MangaBakaRating) -> Self {
        match value {
            MangaBakaRating::Safe => ContentRating::Safe,
            MangaBakaRating::Suggestive => ContentRating::Suggestive,
            MangaBakaRating::Erotica => ContentRating::Erotica,
            MangaBakaRating::Pornographic => ContentRating::Pornographic,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaCover {
    pub raw: CoverRaw,
    pub x150: CoverScaled,
    pub x250: CoverScaled,
    pub x350: CoverScaled,
}

#[derive(Debug, serde::Deserialize)]
pub struct CoverRaw {
    pub uri: Option<String>,
    /// Optional
    pub blurhash: Option<String>,
    /// Optional
    pub format: Option<String>,
    /// Optional
    pub height: Option<i32>,
    /// Optional
    pub size: Option<i32>,
    /// Optional
    pub thumbhash: Option<String>,
    /// Optional
    pub width: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CoverScaled {
    pub x1: Option<String>,
    pub x2: Option<String>,
    pub x3: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaPublisher {
    pub name: Option<String>,
    pub note: Option<String>,
    pub r#type: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaRelationships {
    /// Optional
    pub adaptation: Option<Vec<MangaBakaId>>,
    /// Optional
    pub alternative: Option<Vec<MangaBakaId>>,
    /// Optional
    pub main_story: Option<Vec<MangaBakaId>>,
    /// Optional
    pub other: Option<Vec<MangaBakaId>>,
    /// Optional
    pub prequel: Option<Vec<MangaBakaId>>,
    /// Optional
    pub sequel: Option<Vec<MangaBakaId>>,
    /// Optional
    pub side_story: Option<Vec<MangaBakaId>>,
    /// Optional
    pub spinoff: Option<Vec<MangaBakaId>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SeriesRelationship {
    chronology: Chronology,
    id: String,
    is_manual: bool,
    note: Option<String>,
    #[serde(skip)]
    /// Deprecated
    _published_start_date: (),
    pub relation_type: RelationType,
    pub to_series_id: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chronology {
    Narrative,
    Release,
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    Adaptation,
    Alternative,
    Cameo,
    CharacterFocus,
    Compilation,
    Contains,
    Crossover,
    Expansion,
    Main,
    Other,
    Parent,
    Parody,
    Prequel,
    Reboot,
    Remake,
    Sequel,
    Series,
    SideStory,
    Source,
    SpinOff,
    Summary,
    Uncollected,
}

impl From<RelationType> for crate::media::RelationType {
    fn from(value: RelationType) -> Self {
        match value {
            RelationType::Adaptation => crate::media::RelationType::Adaptation,
            RelationType::Alternative => crate::media::RelationType::Alternative,
            RelationType::Cameo => crate::media::RelationType::Cameo,
            RelationType::CharacterFocus => crate::media::RelationType::CharacterFocus,
            RelationType::Compilation => crate::media::RelationType::Compilation,
            RelationType::Contains => crate::media::RelationType::Contains,
            RelationType::Crossover => crate::media::RelationType::Crossover,
            RelationType::Expansion => crate::media::RelationType::Expansion,
            RelationType::Main => crate::media::RelationType::Main,
            RelationType::Other => crate::media::RelationType::Other,
            RelationType::Parent => crate::media::RelationType::Parent,
            RelationType::Parody => crate::media::RelationType::Parody,
            RelationType::Prequel => crate::media::RelationType::Prequel,
            RelationType::Reboot => crate::media::RelationType::Reboot,
            RelationType::Remake => crate::media::RelationType::Remake,
            RelationType::Sequel => crate::media::RelationType::Sequel,
            RelationType::Series => crate::media::RelationType::Series,
            RelationType::SideStory => crate::media::RelationType::SideStory,
            RelationType::Summary => crate::media::RelationType::Summary,
            RelationType::Source => crate::media::RelationType::Source,
            RelationType::SpinOff => crate::media::RelationType::SpinOff,
            RelationType::Uncollected => crate::media::RelationType::Uncollected,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaSource {
    pub anilist: TitleSource,
    pub anime_news_network: TitleSource,
    pub anime_planet: TitleSource,
    pub kitsu: TitleSource,
    pub manga_updates: TitleSource,
    pub my_anime_list: TitleSource,
    pub shikimori: TitleSource,
}

#[derive(Debug, serde::Deserialize)]
pub struct TitleSource {
    pub id: Option<TitleSourceId>,
    pub rating: Option<f32>,
    /// Optional
    pub rating_normalized: Option<f32>,
}

#[derive(Debug, serde::Deserialize)]
pub enum TitleSourceId {
    #[serde(untagged)]
    Int(u32),
    #[serde(untagged)]
    Slug(String),
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MangaBakaState {
    Active,
    Merged,
    Deleted,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MangaBakaStatus {
    Cancelled,
    Completed,
    Hiatus,
    Releasing,
    Unknown,
    Upcoming,
}

impl From<MangaBakaStatus> for MediaStatus {
    fn from(value: MangaBakaStatus) -> Self {
        match value {
            MangaBakaStatus::Cancelled => MediaStatus::Cancelled,
            MangaBakaStatus::Completed => MediaStatus::Completed,
            MangaBakaStatus::Hiatus => MediaStatus::Hiatus,
            MangaBakaStatus::Releasing => MediaStatus::Releasing,
            MangaBakaStatus::Unknown => MediaStatus::Unknown,
            MangaBakaStatus::Upcoming => MediaStatus::Upcoming,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MangaBakaSeriesType {
    Manga,
    Novel,
    Manhwa,
    Manhua,
    Oel,
    Other,
}

impl From<MangaBakaSeriesType> for MangaFormat {
    fn from(value: MangaBakaSeriesType) -> Self {
        match value {
            MangaBakaSeriesType::Manga => MangaFormat::Manga,
            MangaBakaSeriesType::Novel => MangaFormat::LightNovel,
            MangaBakaSeriesType::Manhwa => MangaFormat::Manhwa,
            MangaBakaSeriesType::Manhua => MangaFormat::Manhua,
            MangaBakaSeriesType::Oel => MangaFormat::Oel,
            MangaBakaSeriesType::Other => MangaFormat::Other,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaTag {
    pub content_rating: MangaBakaRating,
    pub description: Option<String>,
    pub id: u32,
    pub is_spoiler: bool,
    pub level: i32,
    pub name: String,
    pub name_path: String,
    pub parent_id: Option<u32>,
    pub series_count: i32,
    pub implied_by_tag_ids: Vec<i32>,
    pub is_explicit: bool,
    pub is_genre: bool,
    pub merged_with: Option<i32>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct MangaBakaPublication {
    pub end_date: Option<String>,
    pub end_date_is_estimated: Option<bool>,
    pub start_date: Option<String>,
    pub start_date_is_estimated: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MangaBakaTitle {
    // @todo: change this to an enum of language codes (bleh)
    pub language: String,
    pub title: String,
    pub traits: Vec<TitleTrait>,
    pub is_primary: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TitleTrait {
    Official,
    Native,
    Alternative,
}

impl From<TitleTrait> for crate::media::TitleTrait {
    fn from(value: TitleTrait) -> Self {
        match value {
            TitleTrait::Official => crate::media::TitleTrait::Official,
            TitleTrait::Native => crate::media::TitleTrait::Native,
            TitleTrait::Alternative => crate::media::TitleTrait::Alternative,
        }
    }
}
