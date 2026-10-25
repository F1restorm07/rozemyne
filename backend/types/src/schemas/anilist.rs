use backend_graphql::anilist as schema;

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct FuzzyDate {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
}

#[derive(Debug, cynic::Scalar)]
/// 8 digit long date integer (YYYYMMDD) (e.g. 08.07.2016 -> 20160807)
/// Unknown dates are represented by 0
pub struct FuzzyDateInt(i32);

// ----- Media -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Media")]
pub struct AnimeInfo {
    pub id: i32,
    pub title: Option<MediaTitle>,
    pub synonyms: Option<Vec<Option<String>>>,
    pub description: Option<String>,
    pub format: Option<MediaFormat>,
    #[arguments(version: 2)]
    pub status: Option<MediaStatus>,
    pub start_date: Option<FuzzyDate>,
    pub end_date: Option<FuzzyDate>,
    pub season: Option<MediaSeason>,
    pub season_year: Option<i32>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    #[arguments(version: 3)]
    pub source: Option<MediaSource>,
    pub cover_image: Option<MediaCoverImage>,
    pub banner_image: Option<String>,
    pub tags: Option<Vec<Option<MediaTag>>>,
    pub is_adult: Option<bool>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Media")]
pub struct MangaInfo {
    pub id: i32,
    pub title: Option<MediaTitle>,
    pub synonyms: Option<Vec<Option<String>>>,
    pub description: Option<String>,
    pub format: Option<MediaFormat>,
    #[arguments(version: 2)]
    pub status: Option<MediaStatus>,
    pub start_date: Option<FuzzyDate>,
    pub end_date: Option<FuzzyDate>,
    pub season: Option<MediaSeason>,
    pub season_year: Option<i32>,
    pub volumes: Option<i32>,
    pub chapters: Option<i32>,
    #[arguments(version: 3)]
    pub source: Option<MediaSource>,
    pub cover_image: Option<MediaCoverImage>,
    pub banner_image: Option<String>,
    pub tags: Option<Vec<Option<MediaTag>>>,
    pub is_adult: Option<bool>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Media")]
pub struct MediaRelations {
    pub relations: Option<MediaConnection>,
    pub characters: Option<CharacterConnection>,
    pub staff: Option<StaffConnection>,
    pub studios: Option<StudioConnection>,
}

#[derive(Debug, Clone, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct MediaTitle {
    pub romaji: Option<String>,
    pub native: Option<String>,
    pub english: Option<String>,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum MediaFormat {
    Tv,
    TvShort,
    Movie,
    Manga,
    OneShot,
    Novel,
    Music,
    Ona,
    Ova,
    Special,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum MediaStatus {
    Finished,
    Releasing,
    NotYetReleased,
    Cancelled,
    Hiatus,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum MediaSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum MediaSource {
    Anime,
    Manga,
    LightNovel,
    VisualNovel,
    WebNovel,
    Novel,
    Comic,
    Doujinshi,
    LiveAction,
    MultimediaProject,
    PictureBook,
    Game,
    VideoGame,
    Original,
    Other,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct MediaCoverImage {
    medium: Option<String>,
    large: Option<String>,
    extra_large: Option<String>,
    /// average hex color of image
    color: Option<String>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct MediaTag {
    id: i32,
    name: String,
    description: Option<String>,
    category: Option<String>,
    is_general_spoiler: Option<bool>,
    is_media_spoiler: Option<bool>,
    is_adult: Option<bool>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct MediaConnection {
    pub edges: Option<Vec<Option<MediaEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaConnection")]
pub struct StaffMediaConnection {
    pub edges: Option<Vec<Option<StaffMediaEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaConnection")]
pub struct CharacterMediaConnection {
    pub edges: Option<Vec<Option<CharacterMediaEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaConnection")]
pub struct StudioMediaConnection {
    pub edges: Option<Vec<Option<StudioMediaEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct MediaEdge {
    pub id: Option<i32>,
    #[arguments(version: 2)]
    pub relation_type: Option<MediaRelation>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaEdge")]
pub struct StaffMediaEdge {
    pub id: Option<i32>,
    pub characters: Option<Vec<Option<Character>>>,
    pub staff_role: Option<String>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaEdge")]
pub struct CharacterMediaEdge {
    pub id: Option<i32>,
    pub character_role: Option<CharacterRole>,
    pub character_name: Option<String>,
    pub voice_actors: Option<Vec<Option<Staff>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaEdge")]
pub struct StudioMediaEdge {
    pub id: Option<i32>,
    pub is_main_studio: bool,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum MediaRelation {
    Prequel,
    Sequel,
    Source,
    Adaptation,
    Parent,
    Alternative,
    Character,
    Compilation,
    Summary,
    Contains,
    SideStory,
    SpinOff,
    Other,
}

// ----- Character -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct Character {
    id: i32,
    pub name: Option<CharacterName>,
    pub description: Option<String>,
    pub image: Option<CharacterImage>,
    pub gender: Option<String>,
    pub date_of_birth: Option<FuzzyDate>,
    pub age: Option<String>,
    pub media: Option<MediaConnection>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct CharacterName {
    first: Option<String>,
    middle: Option<String>,
    last: Option<String>,
    full: Option<String>,
    native: Option<String>,
    alternative: Option<Vec<Option<String>>>,
    alternative_spoiler: Option<Vec<Option<String>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct CharacterImage {
    large: Option<String>,
    medium: Option<String>,
}

#[derive(Debug, cynic::Enum)]
#[cynic(schema = "anilist")]
pub enum CharacterRole {
    Main,
    Supporting,
    Background,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct CharacterConnection {
    edges: Option<Vec<Option<CharacterEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct CharacterEdge {
    // node: Option<Character>,
    id: Option<i32>,
    role: Option<CharacterRole>,
    name: Option<String>,
    voice_actors: Option<Vec<Option<Staff>>>,
    voice_actor_roles: Option<Vec<Option<StaffRoleType>>>,
    // media: Option<Vec<Option<Media>>>,
}

// ----- Staff -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct Staff {
    id: i32,
    name: Option<StaffName>,
    description: Option<String>,
    image: Option<StaffImage>,
    gender: Option<String>,
    date_of_birth: Option<FuzzyDate>,
    date_of_death: Option<FuzzyDate>,
    age: Option<i32>,
    years_active: Option<Vec<Option<i32>>>,
    home_town: Option<String>,
    staff_media: Option<MediaConnection>,
    characters: Option<CharacterConnection>,
    /// same as characters with media as nodes instead of characters
    character_media: Option<MediaConnection>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StaffName {
    first: Option<String>,
    middle: Option<String>,
    last: Option<String>,
    full: Option<String>,
    native: Option<String>,
    alternative: Option<Vec<Option<String>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StaffImage {
    large: Option<String>,
    medium: Option<String>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StaffRoleType {
    voice_actor: Option<Staff>,
    role_notes: Option<String>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StaffConnection {
    edges: Option<Vec<Option<StaffEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StaffEdge {
    node: Option<Staff>,
    role: Option<String>,
}

// ----- Studios -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StudioConnection {
    edges: Option<Vec<Option<StudioEdge>>>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct StudioEdge {
    node: Option<Studio>,
    is_main: bool,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct Studio {
    id: i32,
    name: String,
    is_animation_studio: bool,
    media: Option<MediaConnection>,
}

// ----- Airing -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct AiringSchedule {
    id: i32,
    airing_at: i32,
    time_until_airing: i32,
    episode: i32,
    media_id: i32,
}

// ----- Pages -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Page", variables = "SearchRequest")]
pub struct MediaSearchPage {
    page_info: Option<PageInfo>,
    #[arguments(search: $search, type: $ty)]
    pub media: Option<Vec<Option<MediaSearchItem>>>,
}

#[derive(Debug, Clone, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Media")]
pub struct MediaSearchItem {
    pub id: i32,
    pub title: Option<MediaTitle>,
}

#[derive(Debug, Clone, cynic::Enum)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "MediaType")]
pub enum MediaType {
    Anime,
    Manga,
}

// ----- Queries -----

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
pub struct PageInfo {
    per_page: Option<i32>,
    current_page: Option<i32>,
    has_next_page: Option<bool>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SearchRequest")]
pub struct MediaSearchQuery {
    #[cynic(rename = "Page")]
    pub page: Option<MediaSearchPage>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SeriesRequest")]
pub struct AnimeInfoQuery {
    #[arguments(id: $id, type: "ANIME")]
    #[cynic(rename = "Media")]
    pub media: Option<AnimeInfo>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SeriesRequest")]
pub struct MangaInfoQuery {
    #[arguments(id: $id, type: "MANGA")]
    #[cynic(rename = "Media")]
    pub media: Option<MangaInfo>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SeriesRequest")]
pub struct MediaRelationsQuery {
    #[arguments(id: $id)]
    #[cynic(rename = "Media")]
    pub media: Option<MediaRelations>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SeriesRequest")]
pub struct CharacterQuery {
    #[arguments(id: $id)]
    #[cynic(rename = "Character")]
    pub character: Option<Character>,
}

#[derive(Debug, cynic::QueryFragment)]
#[cynic(schema = "anilist")]
#[cynic(graphql_type = "Query", variables = "SeriesRequest")]
pub struct StaffQuery {
    #[arguments(id: $id)]
    #[cynic(rename = "Staff")]
    pub staff: Option<Staff>,
}

#[derive(Debug, cynic::QueryVariables)]
pub struct SearchRequest {
    pub search: Option<String>,
    pub ty: Option<MediaType>,
}

#[derive(Debug, cynic::QueryVariables)]
pub struct SeriesRequest {
    pub id: i32,
}
