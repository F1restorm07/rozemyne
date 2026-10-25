use bevy_ecs::component::Component;
use derive_builder::Builder;
use enumset::{EnumSet, EnumSetType};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::Deserialize;
use strum::EnumString;

pub mod data_source;
pub mod relationships;
pub mod query;

#[derive(Debug, Component)]
pub struct ItemUuid(pub uuid::Uuid);
// a file's reference to a media
// @todo: implement a relationship connecting files and media (including episodes and whatnot)
#[derive(Component, Clone)]
pub struct MediaId(pub uuid::Uuid);

#[derive(Debug, Component, EnumSetType, EnumString, Hash, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub enum MediaType {
    Anime,
    Manga,
    Novel,
    Music,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuzzyDate {
    pub year: Option<i16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

#[derive(Component)]
pub struct MediaEpisodeAmount(pub u32);
/// episode duration in minutes
#[derive(Component)]
pub struct MediaEpisodeDuration(pub u32);

#[derive(Component)]
pub struct MediaVolumeAmount(pub u32);
#[derive(Component)]
pub struct MediaChapterAmount(pub u32);
#[derive(Component)]
pub struct MediaSeasonAmount(pub u32);

#[derive(Component)]
pub struct MediaDescription(pub String);

#[derive(Debug, Clone, Component)]
pub struct MediaAirDate {
    pub start: Option<FuzzyDate>,
    pub end: Option<FuzzyDate>,
}

#[derive(Debug, Clone, Component)]
pub struct MediaAirSeason {
    pub year: u32,
    pub season: AirSeason,
}
#[derive(Debug, Clone, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AirSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

#[derive(Debug, Clone, Component, TryFromPrimitive, IntoPrimitive, EnumString)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
pub enum MediaStatus {
    Unknown,
    Upcoming,
    Releasing,
    Completed,
    Cancelled,
    Hiatus,
}

// @todo: express title synonyms (AniList)
// ^^^ i can express them with just the title field, leaving the traits empty and language `None`
#[derive(Debug, Clone, Component)]
pub struct MediaTitles(pub Vec<MediaTitle>);
#[derive(Debug, Clone, PartialEq)]
pub struct MediaTitle {
    pub title: String,
    // two letter language codes (with exceptions -> ja-Latn)
    pub language: Option<String>,
    pub traits: EnumSet<TitleTrait>,
}
#[derive(Debug, EnumSetType)]
pub enum TitleTrait {
    Native,
    Official,
    Alternative,
    Primary // the primary title for this language
}

#[derive(Debug, Clone, Component, TryFromPrimitive, IntoPrimitive, EnumString)]
#[repr(u8)]
#[strum(serialize_all = "lowercase")]
pub enum MediaSource {
    Other,
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
}

#[derive(Debug, Clone, Component, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AnimeFormat {
    Other,
    TVShow,
    TVShort,
    Movie,
    Special,
    Ova,
    Ona,
}

#[derive(Debug, Clone, Component, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum MangaFormat {
    Other,
    Manga,
    Manhwa,
    Manhua,
    LightNovel,
    Oel, // original english language
}

#[derive(Debug, Clone, Component)]
pub enum ContentRating {
    Safe,
    Suggestive,
    Erotica,
    Pornographic,
}

// @todo: convert these into many-to-many relationships
#[derive(Component)]
pub struct MediaRelations(pub Vec<MediaRelation>);
#[derive(Component)]
pub struct MediaCharacters(pub Vec<Character>);
#[derive(Component)]
pub struct MediaStaff(pub Vec<MediaStaffMember>);
#[derive(Component)]
pub struct MediaStudios(pub Vec<Studio>);
#[derive(Component)]
pub struct MediaGenres(pub Vec<String>);

#[derive(Debug, Clone, Builder)]
pub struct MediaStaffMember {
    // single id as every data source may return the name staff with different ids
    // this will get validated and grouped by source id as the media is inserted into the database (or when it gets built)
    // source_id: SourceId,
    #[builder(default)]
    source_id: Option<SourceId>,
    pub name: String,
    #[builder(default)]
    pub desc: Option<String>,
    #[builder(default)]
    pub age: Option<u32>,
    #[builder(default)]
    pub birth_date: Option<FuzzyDate>,
    #[builder(default)]
    pub gender: Option<Gender>,
    #[builder(default)]
    pub hometown: Option<String>,
    #[builder(default)]
    pub years_active: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Gender {
    Male,
    Female,
    Agender,
    NonBinary,
    Intersex,
    Other,
}

#[derive(Debug, Clone)]
pub struct Studio {
    source_id: SourceId,
    name: String,
    is_animation_studio: bool,
}

#[derive(Debug, Clone)]
pub struct MediaRelation {
    pub target_id: SourceId,
    pub target_type: MediaType,
    pub rel_type: RelationType,
}
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    source_id: SourceId,
    desc: Option<String>,
    gender: Option<String>,
    // anilist adds extra info to the age (e.g. age range, ages across arcs/time skips, etc)
    age: Option<String>,
    birth_date: Option<FuzzyDate>,
    // @todo: find a way to link to the base character, without knowing who the base character is
    is_base: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum CharacterRole {
    Main,
    Supporting,
    Background,
}

#[derive(Debug, Component)]
pub struct MediaTags(pub Vec<MediaTag>);
#[derive(Debug, Builder)]
pub struct MediaTag {
    pub name: String,
    #[builder(default)]
    pub description: Option<String>,
    #[builder(default)]
    pub category: Option<String>,
    pub is_spoiler: bool,
    pub content_rating: ContentRating,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct SourceIds(pub Vec<SourceId>);
#[derive(Debug, Clone, PartialEq)]
pub struct SourceId {
    pub source: String,
    // if no concrete id exists for a given datapoint (ex. staff in MangaBaka), no source id is stored
    // although, an entity is still generated with only the name
    pub id: u32,
}
