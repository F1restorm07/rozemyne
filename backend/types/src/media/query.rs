use bevy_ecs::component::Component;

use super::{AirSeason, AnimeFormat, FuzzyDate, MangaFormat, MediaSource, MediaStatus};

// @todo: implement `In`, `Not`, `GreaterThan`, and `LessThan` query types

#[derive(Component)]
pub struct IsMature;
#[derive(Component)]
pub struct IsLicensed;
#[derive(Component)]
pub struct HasAdaptation;

#[derive(Debug, Clone, Component)]
pub struct MediaTitleQuery(pub String);

#[derive(Component)]
pub struct MediaSeasonQuery(pub u32);
#[derive(Component)]
pub struct MediaEpisodeQuery(pub u32);
#[derive(Component)]
pub struct MediaChapterQuery(pub u32);
#[derive(Component)]
pub struct MediaVolumeQuery(pub u32);

#[derive(Component)]
pub struct MediaSourceQuery(pub MediaSource);
#[derive(Component)]
pub struct MediaStatusQuery(pub MediaStatus);

#[derive(Component)]
pub struct MediaGenreQuery(pub String);

#[derive(Component)]
pub struct MediaStaffQuery(pub String);
#[derive(Component)]
pub struct MediaStudioQuery(pub String);

#[derive(Component)]
pub struct MediaStartDateQuery(pub FuzzyDate);
#[derive(Component)]
pub struct MediaEndDateQuery(pub FuzzyDate);
#[derive(Component)]
pub struct MediaAirSeasonQuery(pub AirSeason);

#[derive(Component)]
pub struct AnimeFormatQuery(pub AnimeFormat);
#[derive(Component)]
pub struct MangaFormatQuery(pub MangaFormat);
