// command line argument layout
// ******************************
// - server >> run the server process (includes metadata + http servers)
// - import >> performs the whole import process for a given path
// - search >> query `rozemyne` for the given media, printing the full info page
// - add-import-folder >> add the given import folder
// - add-dest-folder >> add the given destination folder
//
// - files
// -- hash >> hash the given file into a 256-bit `highway` hash
// -- parse-title >> parse the given filename into `TitlePart`s
// -- rename >> rename the given file (provide a rule engine instance)
// -- move >> move the given file (provide a rule engine instance)
//
// - data-source
// -- search >> query the provided data source(s) with the given query
// --- filters >> a series of nested queries on which to search (ex. title="rezero",year=2024)
// -- character >> search for the given character with the provided data source(s)
// -- <etc>
//
// - query
// -- search >> query the database for the given item
// --- file >> search for the given filename
// --- media >> search for the info of a given media title
// --- character >> search for the given character
// --- staff
// --- studio
// --- relations >> get all the media relations of a given item

use std::path::PathBuf;

use crate::configuration::Settings;
use crate::files::{self, FileLoadingPlugin};
use crate::files::renamer::PatternEngine;
use backend_types::cli::{Args, CliExternalQuery, CliFileAction, CliInternalQuery, ExternalQueryCmd, FilesCmd, InternalQueryCmd, QueryType};
use backend_types::files::{MediaFile, ParsedFilename};
use backend_types::media::query::{MediaGenreQuery, MediaTitleQuery};
use backend_types::media::{ItemUuid, MediaId, MediaTitles, MediaType, TitleTrait};
use backend_types::media::data_source::{DataQueryType, Media, MediaExternalRequest, MediaInternalRequest, MediaInternalResponse, NoInternalMedia};
use backend_types::parser::token::Lexer;
use backend_types::renamer::{Filterable, Value};
use bevy_app::AppExit;
use bevy_ecs::component::Component;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::observer::On;
use bevy_ecs::query::{With, Without};
use bevy_ecs::resource::{IsResource, Resource};
use bevy_ecs::system::{Commands, Query, Res, Single};
use bevy_ecs::world::EntityRef;
use bpaf::{Bpaf, Parser};
use tabled::Table;
use tabled::builder::Builder;
use tabled::settings::style::{HorizontalLine, VerticalLine};
use tabled::settings::themes::BorderCorrection;
use tabled::settings::{Merge, Panel, Style};
use yansi::Paint;

pub struct CliPlugin;

impl bevy_app::Plugin for CliPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let args = backend_types::cli::cli_args().run();
        if let Args::Server = args {
            app.add_plugins(FileLoadingPlugin);
        }
        app.insert_resource(args);
        app.add_systems(bevy_app::Startup, build_action);
        app.add_systems(bevy_app::PreUpdate, cli_files);
        // insert into `PostUpdate` since most to all other systems operate in `Update`
        // @todo: move `cli_files` into `PreUpdate` to allow for querying the ECS system for renaming
        app.add_systems(bevy_app::PostUpdate, (cli_internal_query, cli_external_query, cli_rename));
        app.add_observer(cli_internal_fail);
    }
}

fn build_action(
    cli_args: Res<Args>,
    settings: Res<Settings>,
    mut cmds: Commands,
) {
    match cli_args.into_inner() {
        Args::Files(args) => {
            cmds.spawn(CliFileAction {
                file: MediaFile::new(&args.path, settings.hash_buffer_size() as usize, "".to_string()),
                cmd: args.cmd.clone(),
            });
        }
        Args::Query(args) => {
            match args.ty {
                QueryType::Internal { ref cmd } => {
                    match cmd {
                        InternalQueryCmd::Search => {
                            let media_req =
                                MediaInternalRequest::new(DataQueryType::Search, args.filters.media_type);
                            cmds.spawn(CliInternalQuery { id: media_req.id() });
                            cmds.spawn((
                                media_req, MediaTitleQuery(args.filters.title.clone()),
                            ));
                        }
                        _ => {}
                    }
                }
                QueryType::External { ref cmd, ref sources } => {
                    match cmd {
                        ExternalQueryCmd::Search => {
                            // @todo: when dispatching requests, *only* send request types matching the original request type
                            let mut media_req =
                                MediaExternalRequest::new(DataQueryType::Search, args.filters.media_type);
                            if !sources.is_empty() { media_req = media_req.with_sources(sources.to_vec()); }

                            // how can i hijack the external query process such that
                            // the queries are not converted into media entries?
                            cmds.spawn(CliExternalQuery { id: media_req.id() });
                            let mut cmd = cmds.spawn((
                                media_req, MediaTitleQuery(args.filters.title.clone()),
                            ));

                            if let Some(ref genre) = args.filters.genre {
                                cmd.insert(MediaGenreQuery(genre.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Args::Server => {}
    }
}

fn cli_files(
    // engine: Res<PatternEngine>,
    action: Single<&CliFileAction>,
    mut exit: MessageWriter<AppExit>,
    mut cmds: Commands,
) {
    // let hor_separator = "┈┈・୨ ✦ ୧・┈┈┈┈・୨ ✦ ୧・┈┈┈┈・୨ ✦ ୧・┈┈┈┈・୨ ✦ ୧・┈┈";
    // let hor_separator = "𓆝 𓆟 𓆞 𓆝 𓆟 𓆞 𓆝 𓆟 𓆞 𓆝 𓆟 𓆞 𓆝";
    // let hor_separator = "✦ . 　⁺ 　 . ✦ . 　⁺ 　 . ✩₊˚.⋆☾⋆⁺₊✧ . 　⁺ 　 . ✦ . 　⁺ 　 . ✦";
    // let hor_separator = "✦ . 　⁺ 　 . ✦ . 　⁺ 　 . ⊹₊˚‧︵‿₊⊱·✶·⊰₊‿︵‧˚₊⊹ . 　⁺ 　 . ✦ . 　⁺ 　 . ✦".bright_black();
    let action = action.into_inner();
    match &action.cmd {
        FilesCmd::Hash => {
            let hex = const_hex::encode(action.file.hash());
            // println!(". ₊ ⊹ . ₊˖ . ₊. ₊ ⊹ . ₊˖ . ₊ {} . ₊ ⊹ . ₊˖ . ₊. ₊ ⊹ . ₊˖ . ₊", "File Hash".blue().bold());
            println!("✦ . 　⁺ 　 . ✦ . 　⁺ 　 . {} . 　⁺ 　 . ✦ . 　⁺ 　 . ✦", "File Hash".blue().bold());
            // println!("{hor_separator}");
            println!("{}", hex.green());
        }
        cmd => {
            let filename = action.file.path().file_name().unwrap().to_str().unwrap();
            let tokens = Lexer::new(filename).into_tokens();
            let title_parts = files::parser::parse(tokens);
            let parsed = ParsedFilename::new(&title_parts);

            // @todo: query for media? -> needed to build the full engine context
            // ^^ i can stage this further by pinging the ECS and using a separate rename system to handle the results
            match cmd {
                FilesCmd::ParseTitle => {
                    let info_table = create_parsed_info_table(&parsed);
                    let metadata_table = create_parsed_metadata_table(&parsed);
                    let flags_table = create_parsed_flags_table(&parsed);

                    println!("✦ . 　⁺ 　 . ✦ . 　⁺ 　 . {} ⋆⭒˚.⋆. {} . 　⁺ 　 . ✦ . 　⁺ 　 . ✦", "Parsed Information".blue().bold(), filename.green());
                    // println!("{hor_separator}");
                    println!("{info_table}");
                    println!("{metadata_table}");
                    println!("{flags_table}");
                }
                FilesCmd::Rename { media_type, .. } | FilesCmd::Move { media_type, .. } => {
                    // ping media with the parsed title parts
                    // ^^ send as `MediaRequest`
                    // handle actual renaming + moving in a different system
                    // ^^ do i want a internal or external request?
                    // ^^ for requests, i need to glean a media type
                    let media_req =
                        MediaInternalRequest::new(DataQueryType::Search, *media_type);
                    cmds.spawn((
                        media_req.clone(), MediaTitleQuery(parsed.title.clone()),
                    ));
                    cmds.spawn((action.file.clone(), parsed, CliInternalQuery { id: media_req.id() }));
                    println!("sending out request `{:?}`", media_req.id());
                }
                _ => {}
            }
        }
    }
    exit.write(AppExit::Success);
}

// const TABLE_STYLE: Style<_, _, _, _, _, _, 1, 1> = Style::modern_rounded()
//     .horizontals([(1, HorizontalLine::inherit(Style::modern()).intersection('┬'))])
//     .verticals([(1, VerticalLine::inherit(Style::modern()).top('─'))])
//     .remove_horizontal()
//     .remove_vertical();


fn create_parsed_info_table(parsed: &ParsedFilename) -> Table {
    // general information
    let mut info_builder = Builder::new();
    info_builder.push_record(vec!["Title", &parsed.title]);

    if let Some(ref title) = parsed.episode_title {
        info_builder.push_record(vec!["Episode Title", &title]);
    }
    if let Some(szn) = parsed.season {
        info_builder.push_record(vec!["Season", &format!("{szn}")]);
    }
    if let Some(ep) = parsed.episode {
        info_builder.push_record(vec!["Episode", &format!("{ep}")]);
    }
    if let Some(ch) = parsed.chapter {
        info_builder.push_record(vec!["Chapter", &format!("{ch}")]);
    }
    if let Some(vol) = parsed.volume {
        info_builder.push_record(vec!["Volume", &format!("{vol}")]);
    }

    let mut table = info_builder.build();
    table
        .with(Panel::header("Info"))
        .with(BorderCorrection::span())
        .with(Style::modern_rounded()
            .horizontals([(1, HorizontalLine::inherit(Style::modern()).intersection('┬'))])
            .verticals([(1, VerticalLine::inherit(Style::modern()).top('─'))])
            .remove_horizontal()
            .remove_vertical()
        );
    table
}
// @todo: support grouping *all* parsed metadata sections
// or, sort them by quality and select the highest one
fn create_parsed_metadata_table(parsed: &ParsedFilename) -> Table {
    let mut builder = Builder::new();

    if let Some(ref g) = parsed.release_group {
        builder.push_record(vec!["Release Group", &g]);
    }
    if let Some(y) = parsed.year {
        builder.push_record(vec!["Year", &format!("{y}")]);
    }
    if let Some(ref s) = parsed.service {
        builder.push_record(vec!["Service", &s]);
    }
    if let Some(ref s) = parsed.rip_source {
        builder.push_record(vec!["Rip Source", &s]);
    }
    if let Some(ref r) = parsed.resolution {
        builder.push_record(vec!["Resolution", &r]);
    }
    if let Some(ref c) = parsed.audio_codec {
        builder.push_record(vec!["Audio Codec", &c]);
    }
    if let Some(ref c) = parsed.audio_channels {
        builder.push_record(vec!["Audio Channels", &c]);
    }
    if let Some(ref c) = parsed.video_codec {
        builder.push_record(vec!["Video Codec", &c]);
    }
    if !parsed.languages.is_empty() {
        let mut row = vec!["Languages"];
        row.extend(parsed.languages.iter().map(|l| l.as_str_short()));
        builder.push_record(row);
    }
    if let Some(ref e) = parsed.file_extension {
        builder.push_record(vec!["File Extension", &e]);
    }
    if let Some(v) = parsed.version {
        builder.push_record(vec!["Version", &format!("{v}")]);
    }

    let mut table = builder.build();
    if !table.is_empty() {
        table.with(Panel::header("Metadata"));
    }

    table
        .with(Merge::horizontal())
        .with(BorderCorrection::span())
        .with(Style::modern_rounded()
            .horizontals([(1, HorizontalLine::inherit(Style::modern()).intersection('┬'))])
            .verticals([(1, VerticalLine::inherit(Style::modern()).top('─'))])
            .remove_horizontal()
            .remove_vertical()
        );
    table
}

fn create_parsed_flags_table(parsed: &ParsedFilename) -> Table {
    let mut builder = Builder::new();

    for flag in parsed.keyword_flags {
        builder.push_record(vec![&format!("{flag}")]);
    }

    let mut table = builder.build();
    if !table.is_empty() {
        table.with(Panel::header("Flags"));
    }
    table
        .with(Merge::horizontal())
        .with(BorderCorrection::span())
        .with(Style::modern_rounded()
            .horizontals([(1, HorizontalLine::inherit(Style::modern()).intersection('┬'))])
            .remove_horizontal()
            .remove_vertical()
        );
    table
}

// how can i pull an error out of a system from bevy's error handler
// ^^ this system *always* runs, even when the `dispatch_media_request` system fails
fn cli_internal_query(
    query: Single<&CliInternalQuery>,
    resps: Query<(&MediaInternalResponse, &MediaId)>,
    media: Query<(EntityRef, &ItemUuid), (With<Media>, Without<MediaFile>, Without<IsResource>)>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some((_resp, id)) = resps.iter().find(|(r,_)| r.id() == query.id) {
        let (ent, id) = media.iter().find(|(_, i)| i.0 == id.0).unwrap();
        let title = ent.get::<MediaTitles>().unwrap().0.iter()
            .find(|t|
                (t.language == Some("en".to_string()) || t.language == Some("ja-Latn".to_string()))
                    && (t.traits.contains(TitleTrait::Official)
                        || t.traits.contains(TitleTrait::Primary))
            )
            .unwrap();
        // @todo: point towards the `info` command?? (or display all search results?)
        println!("internal media id: {:?}", id.0);
        println!("media title: {}", title.title);
        println!("all titles: {:?}", ent.get::<MediaTitles>().unwrap());
        exit.write(AppExit::Success);
    }
}

fn cli_internal_fail(
    _fail: On<NoInternalMedia>,
    mut exit: MessageWriter<AppExit>,
) {
    // @todo: include (pretty-print) the query information
    println!("no internal media found for the given query");
    exit.write(AppExit::error());
}

fn cli_external_query(
    query: Single<&CliExternalQuery>,
    media: Query<(&ItemUuid, &MediaTitles), With<Media>>,
    mut exit: MessageWriter<AppExit>,
) {
    // @todo: return *raw* query responses here (one per external source)
    // ^^ this would require me to query data source responses + collect by data source
    if let Some((_id, titles)) = media.iter().find(|(id,_)| id.0 == query.id) {
        let eng_title = titles.0.iter()
            .find(|t|
                (t.language == Some("en".to_string()) || t.language == Some("ja-Latn".to_string()))
                    && (t.traits.contains(TitleTrait::Official)
                        || t.traits.contains(TitleTrait::Primary))
            )
            .unwrap();
        // @todo: prettify this
        println!("media title: {}", eng_title.title);
        exit.write(AppExit::Success);
    } else {
        // @todo: determine an appropriate wait time for data responses (seems to be 3 update ticks before we receive a full-fledged `Media`)
        // ^^ update: this takes 1 update cycle to run (we hit this branch once)
        // ^^ update: after moving the system to `PostUpdate`, this takes a single cycle, and thus we should only hit the branch if an error occurred
        eprintln!("media not found in external sources");
        exit.write(AppExit::error());
    }
}

fn cli_rename(
    settings: Res<Settings>,
    engine: Res<PatternEngine>,
    query: Single<(&mut MediaFile, &ParsedFilename, &CliInternalQuery)>,
    resps: Query<(&MediaInternalResponse, &MediaId)>,
    media: Query<(EntityRef, &ItemUuid), (With<Media>, Without<MediaFile>, Without<IsResource>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let (mut file, file_info, query) = query.into_inner();

    if let Some((_resp, id)) = resps.iter().find(|(r,_)| r.id() == query.id) {
        let (media, _id) = media.iter().find(|(_, i)| i.0 == id.0).unwrap();
        let mut ctx = file_info.get_context();

        let media_titles = media.get::<MediaTitles>().unwrap();
        let romaji_title = media_titles.0.iter().find(|t|
            t.language.as_ref().is_some_and(|t| t == "ja-Latn") && t.traits.contains(TitleTrait::Primary)
        );
        let eng_title = media_titles.0.iter().find(|t|
            t.language.as_ref().is_some_and(|t| t == "en") && t.traits.contains(TitleTrait::Official)
        );

        if let Some(title) = romaji_title {
            ctx.insert("media.title.romaji", title.title.as_str());
        }
        if let Some(title) = eng_title {
            ctx.insert("media.title.english", title.title.as_str());
        }

        ctx.insert("file.name", file_info.title.clone());
        let root_path = settings.media_folders().iter()
            .find(|f| f.id() == file.folder_id());
        let file_path = if let Some(path) = root_path {
            path.destination_path().iter()
                .map(|s| Value::Str(s.to_str().unwrap().into()))
                .collect::<Vec<_>>()
        } else {
            file.path().parent().unwrap().iter()
                .map(|s| Value::Str(s.to_str().unwrap().into()))
                .collect::<Vec<_>>()
        };
        ctx.insert("file.path", file_path);
        ctx.insert("file.extension", file_info.file_extension.as_deref().unwrap_or(""));

        engine.render(&mut ctx);

        let file_path = ctx.get("file.path").unwrap()
            .as_array().unwrap()
            .iter().map(|v| v.as_string().unwrap())
            .collect::<std::path::PathBuf>();
        let file_path = file_path.join(ctx.get("file.name").unwrap().as_string().unwrap())
            .with_extension(ctx.get("file.extension").unwrap().as_string().unwrap());

        std::fs::create_dir_all(file_path.parent().unwrap());
        std::fs::rename(file.path(), &file_path);
        *file.path_mut() = file_path;

        exit.write(AppExit::Success);
    }
}
