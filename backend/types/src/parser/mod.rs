use std::borrow::Cow;
use std::range::Range;

pub mod keyword;
pub mod token;

use keyword::KeywordKind;

use self::keyword::{Language, RipSource};

#[derive(Debug, PartialEq, Clone, Copy, strum::Display)]
#[repr(C)]
pub enum SegmentKind {
    Title,
    EpisodeTitle,

    Season,
    Episode,
    EpisodeRange,
    Volume,
    VolumeRange,
    Chapter,
    Part,
    Date,
    Year,

    Service,
    Source(RipSource),
    Region,
    Encoder,
    Version,

    Language(Language),
    Group,
    Website,
    Dubbed,
    Subbed,

    AudioCodec,
    VideoCodec,
    Audio,
    AudioChannels,
    BitDepth,
    SampleRate,
    Color,
    ThreeD,
    Resolution,
    Genre,
    Size,

    Complete,
    Convert,
    Container,
    Criterion,
    Documentary,
    Downscaled,
    Extended,
    FileExtension,
    Hardcoded,
    Hybrid,
    Internal,
    International,
    OpenMatte,
    Proper,
    Repack,
    Remastered,
    Remux,
    Retail,
    Theatrical,
    Uncut,
    Unrated,
}

#[derive(Debug)]
pub struct Segment<'src> {
    pub kind: SegmentKind,
    pub text: Cow<'src, str>,
    pub pos: Range<usize>
}

pub fn keyword_to_title_part(kywd: KeywordKind) -> SegmentKind {
    match kywd {
        KeywordKind::Service => SegmentKind::Service,
        KeywordKind::Source(s) => SegmentKind::Source(s),
        KeywordKind::Region => SegmentKind::Region,
        KeywordKind::Encoder => SegmentKind::Encoder,
        KeywordKind::Genre => SegmentKind::Genre,
        KeywordKind::Complete => SegmentKind::Complete,
        KeywordKind::Season => SegmentKind::Season,
        KeywordKind::Episode => SegmentKind::Episode,
        KeywordKind::Volume => SegmentKind::Volume,
        KeywordKind::Chapter => SegmentKind::Chapter,
        KeywordKind::Part => SegmentKind::Part,
        KeywordKind::ReleaseVersion => SegmentKind::Version,
        KeywordKind::AudioChannels => SegmentKind::AudioChannels,
        KeywordKind::AudioCodec => SegmentKind::AudioCodec,
        KeywordKind::Language(l) => SegmentKind::Language(l),
        KeywordKind::Dubbed => SegmentKind::Dubbed,
        KeywordKind::Subbed => SegmentKind::Subbed,
        KeywordKind::VideoCodec => SegmentKind::VideoCodec,
        KeywordKind::VideoColor => SegmentKind::Color,
        KeywordKind::VideoColorDepth => SegmentKind::BitDepth,
        KeywordKind::VideoResolution => SegmentKind::Resolution,
        KeywordKind::FileExtension => SegmentKind::FileExtension,
        KeywordKind::Convert => SegmentKind::Convert,
        KeywordKind::Criterion => SegmentKind::Criterion,
        KeywordKind::Documentary => SegmentKind::Documentary,
        KeywordKind::Downscaled => SegmentKind::Downscaled,
        KeywordKind::Extended => SegmentKind::Extended,
        KeywordKind::Hardcoded => SegmentKind::Hardcoded,
        KeywordKind::Hybrid => SegmentKind::Hybrid,
        KeywordKind::Internal => SegmentKind::Internal,
        KeywordKind::International => SegmentKind::International,
        KeywordKind::OpenMatte => SegmentKind::OpenMatte,
        KeywordKind::Proper => SegmentKind::Proper,
        KeywordKind::Remastered => SegmentKind::Remastered,
        KeywordKind::Remux => SegmentKind::Remux,
        KeywordKind::Repack => SegmentKind::Repack,
        KeywordKind::Retail => SegmentKind::Retail,
        KeywordKind::Theatrical => SegmentKind::Theatrical,
        KeywordKind::Uncut => SegmentKind::Uncut,
        KeywordKind::Unrated => SegmentKind::Unrated,
    }
}
