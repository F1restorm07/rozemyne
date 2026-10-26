use std::path::PathBuf;

use bevy_ecs::component::Component;
use bevy_ecs::resource::Resource;
use bpaf::doc::Style;
use bpaf::{Bpaf, Parser};

use crate::files::MediaFile;
use crate::media::{MediaSource, MediaStatus, MediaType};

#[derive(Debug, Clone, Resource, Bpaf)]
#[bpaf(generate(cli_args))]
pub enum Args {
    #[bpaf(command)]
    Files(#[bpaf(external(files))] FilesArgs),
    #[bpaf(command)]
    Query(#[bpaf(external(query))] QueryArgs),
    #[bpaf(command)]
    Server,
}

#[derive(Debug, Clone, Bpaf)]
pub enum FilesCmd {
    /// perform a HighwayHash on the given file's contents
    #[bpaf(command)]
    Hash,
    /// parse the filename of the given file
    #[bpaf(command)]
    ParseTitle,
    /// rename the given file based on pattern rules
    #[bpaf(command)]
    Rename {
        media_type: MediaType
    },
    /// move the given file based on pattern rules
    #[bpaf(command)]
    Move {
        media_type: MediaType,
    },
}
#[derive(Debug, Clone, Bpaf)]
#[bpaf(generate(files))]
pub struct FilesArgs {
    // #[bpaf(positional("PATH"))]
    #[bpaf(long("path"))]
    pub path: PathBuf,
    #[bpaf(external(files_cmd))]
    pub cmd: FilesCmd,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(fallback(ExternalQueryCmd::Search))]
pub enum ExternalQueryCmd {
    /// perform a general search query
    Search,
    /// query for a singular piece of media
    Info,
    /// query for a single character
    Character,
}

// @todo: move the data source command under here
#[derive(Debug, Clone, Bpaf)]
#[bpaf(fallback(InternalQueryCmd::Search))]
pub enum InternalQueryCmd {
    // perform a general search query
    Search,
    // query for a specific file
    File,
    // query for a specific media
    Media,
}

#[derive(Debug, Clone, Bpaf)]
pub enum QueryType {
    #[bpaf(command)]
    Internal {
        #[bpaf(external(internal_query_cmd), group_help("query types"))]
        cmd: InternalQueryCmd
    },
    #[bpaf(command)]
    External {
        #[bpaf(external(external_query_cmd), group_help("query types"))]
        cmd: ExternalQueryCmd,
        #[bpaf(long("source"), argument("SOURCE"), help("source(s) to query"))]
        sources: Vec<String>,
    }
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(generate(query))]
pub struct QueryArgs {
    #[bpaf(external(query_filters), group_help("query filters"))]
    pub filters: QueryFilters,
    #[bpaf(external(query_type))]
    pub ty: QueryType,
}

// @todo: fill this out
#[derive(Debug, Clone)]
pub struct QueryFilters {
    /// query for the given title
    pub title: String,
    // @todo: allow support for multiple media types
    /// filter for data sources supporting the given media type
    pub media_type: MediaType,
    // @todo: allow multiple genres -> many or collection parser?
    /// filter by the given genre
    pub genre: Option<String>,
    pub seasons: Option<u32>,
    pub episodes: Option<u32>,
    pub chapters: Option<u32>,
    pub volumes: Option<u32>,
    pub source: Option<MediaSource>,
    pub status: Option<MediaStatus>,
}

fn tag<T>(name: &'static str, meta: &str, help: &str) -> impl Parser<T>
where
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    bpaf::any("", move |s: String| Some(s.strip_prefix(name)?.to_owned()))
        .metavar(&[(name, Style::Literal), (meta, Style::Metavar)][..])
        .help(help)
        .anywhere()
        .parse(|s| s.parse())
}

fn title() -> impl Parser<String> {
    tag::<String>("title=", "TITLE", "query for the given TITLE")
}
fn media_type() -> impl Parser<MediaType> {
    tag::<MediaType>("media_type=", "MEDIA_TYPE", "filter the query to only the data sources supporting the given MEDIA_TYPE")
}
fn genre() -> impl Parser<Option<String>> {
    tag::<String>("genre=", "GENRE", "filter to the given GENRE")
        .optional()
}
fn seasons() -> impl Parser<Option<u32>> {
    tag("seasons=", "SEASONS", "filter for media containing the given amount of SEASONS")
        .optional()
}
fn episodes() -> impl Parser<Option<u32>> {
    tag("episodes=", "EPISODES", "filter for media containing the given amount of EPISODES")
        .optional()
}
fn chapters() -> impl Parser<Option<u32>> {
    tag("chapters=", "CHAPTERS", "filter for media containing the given amount of CHAPTERS")
        .optional()
}
fn volumes() -> impl Parser<Option<u32>> {
    tag("volumes=", "VOLUMES", "filter for media containing the given amount of VOLUMES")
        .optional()
}
fn source() -> impl Parser<Option<MediaSource>> {
    tag("source=", "SOURCE", "filter for media with the given SOURCE")
        .optional()
}
fn status() -> impl Parser<Option<MediaStatus>> {
    tag("status=", "STATUS", "filter for media with the given STATUS")
        .optional()
}

fn query_filters() -> impl Parser<QueryFilters> {
    bpaf::construct!(QueryFilters {
        title(),
        media_type(),
        genre(),
        seasons(),
        episodes(),
        chapters(),
        volumes(),
        source(),
        status(),
    })
}

#[derive(Component)]
pub struct CliFileAction {
    pub file: MediaFile,
    pub cmd: FilesCmd,
}
#[derive(Component)]
pub struct CliExternalQuery { pub id: uuid::Uuid }
#[derive(Component)]
pub struct CliInternalQuery { pub id: uuid::Uuid }
