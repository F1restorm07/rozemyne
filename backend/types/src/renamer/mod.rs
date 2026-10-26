use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use bevy_asset::{Handle};
use bevy_ecs::resource::Resource;
use interpolator::Formattable;
use mlua::AnyUserData;
use regex::Regex;

use crate::media::{AnimeFormat, MangaFormat};
use crate::parser::keyword::{Language, RipSource};

// use self::condition::Condition;
// use self::filter::{StoredFilter, StoredFunction, StoredTest};
// use self::predicate::Predicate;

// mod token;
// pub mod condition;
// pub mod predicate;
// pub mod filter;

#[derive(Resource)]
pub struct RenamerScript {
    pub script: Handle<crate::lua::ScriptAsset>,
    pub context: mlua::Lua,
}

#[derive(mlua::FromLua, mlua::UserData, Clone)]
pub struct RenamerSettings {
    sanitize_filename: bool,
    rename_file: bool,
    move_file: bool,
}

#[mlua::userdata_impl]
impl RenamerSettings {
    #[lua(meta, infallible)]
    fn __call(_: AnyUserData, sanitize_filename: bool, rename_file: bool, move_file: bool) -> Self {
        Self { sanitize_filename, rename_file, move_file }
    }
}

#[derive(mlua::FromLua, mlua::UserData, Clone)]
pub struct RenamedFile {
    folder_id: String,
    destination_path: PathBuf,
    name: String,
    settings: RenamerSettings
}

impl RenamedFile {
    pub fn file_path(&self) -> &std::path::Path { &self.destination_path }
    pub fn file_name(&self) -> &str { &self.name }
    pub fn folder_id(&self) -> &str { &self.folder_id }

    pub fn rename_file(&self) -> bool { self.settings.rename_file }
    pub fn move_file(&self) -> bool { self.settings.move_file }
    pub fn sanitize_filename(&self) -> bool { self.settings.sanitize_filename }
}

#[mlua::userdata_impl]
impl RenamedFile {
    // @todo: clean this up (with named arguments) -> can i actually create this from a raw table?
    #[lua(meta, infallible)]
    fn __call(_: AnyUserData, folder_id: String, destination_path: PathBuf, name: String, settings: RenamerSettings) -> Self {
        Self { folder_id, destination_path, name, settings }
    }
}

// @todo: fill this out (keywords, bit depth, etc)
#[derive(mlua::UserData)]
pub struct FileMetadata {
    pub folder_id: String,
    pub path: PathBuf,
    pub title: String,
    pub group: Option<String>,
    // i may swap this out for an enumset
    pub languages: Vec<Language>,
    pub resolution: Option<String>,
    pub audio_codec: Option<String>,
    pub video_codec: Option<String>,
    pub service: Option<String>,
    pub rip_source: Option<RipSource>,
    pub bit_depth: Option<String>,

    pub file_version: Option<u32>,
}

#[mlua::userdata_impl]
impl FileMetadata {
    fn filename(&self) -> mlua::Result<std::ffi::OsString> {
        Ok(self.path.file_name().unwrap().to_os_string())
    }
    fn directory(&self) -> mlua::Result<PathBuf> {
        Ok(self.path.parent().unwrap().to_path_buf())
    }
}

// @todo: break out media specific info into `info` attribute
// @todo: break out media agnostic info into other attributes
#[derive(mlua::UserData)]
pub enum MediaMetadata {
    Anime {
        format: AnimeFormat,
        title: String,
        // @todo: add episode ranges (split this into an enum)
        // @todo: add episode types
        episode: u32,
        episode_title: Option<String>,
        explicit: bool,
    },
    Manga {
        format: MangaFormat,
        title: String,
        volume: Option<u32>,
        chapter: Option<u32>,
        volume_title: Option<String>,
        chapter_title: Option<String>,
        explicit: bool,
    },
}

// pub struct State<'eng, 'ctx> {
//     pub context: &'ctx mut Context,
//     pub filters: &'eng HashMap<String, StoredFilter>,
//     pub tests: &'eng HashMap<String, StoredTest>,
//     pub functions: &'eng HashMap<String, StoredFunction>,
//     pub default_formatter: &'eng StoredFilter,
// }
//
// pub struct PatternMatch<'patt> {
//     pattern: &'patt Pattern
// }
//
// impl<'patt> PatternMatch<'patt> {
//     pub fn new(pattern: &'patt Pattern) -> Self {
//         Self { pattern }
//     }
//     pub fn target_buffer(&self) -> &Option<Path> {
//         &self.pattern.target_buffer
//     }
//
//     pub fn render(
//         &self,
//         target_buf: &mut String,
//         state: &State<'_, '_>,
//     ) {
//         self.pattern.render(target_buf, state);
//     }
// }
//
// #[derive(Debug)]
// pub struct Pattern {
//     cond: Option<Condition>,
//     pred: Predicate,
//     // @todo: find a spick and span way of storing target buffers
//     target_buffer: Option<Path>,
// }
//
// impl Pattern {
//     pub fn new(condition: Option<&str>, predicate: &str, target_buffer: Option<&str>) -> Self {
//         Self {
//             cond: condition.map(Condition::compile),
//             pred: Predicate::compile(predicate),
//             target_buffer: target_buffer.map(|b| {
//                 let parser = token::Parser::new(b);
//                 let first_seg = PathSegment::parse(&parser);
//                 Path::parse(&parser, first_seg)
//             }),
//         }
//     }
//     pub fn target_buffer(&self) -> &Option<Path> {
//         &self.target_buffer
//     }
//
//     pub fn is_match(&self, state: &State) -> bool {
//         self.cond.as_ref().map(|c| c.run(state)).unwrap_or(true)
//     }
//     pub fn render(
//         &self,
//         target_buf: &mut String,
//         state: &State<'_, '_>,
//     ) {
//         self.pred.render(target_buf, state);
//     }
// }
//
// pub struct FormatPattern {
//     cond: Option<Condition>,
//     fmt_str: String,
//     target: Path,
// }
//
// impl FormatPattern {
//     pub fn new(cond: Option<&str>, fmt_str: String, target: &str) -> Self {
//         Self {
//             cond: cond.map(Condition::compile),
//             fmt_str,
//             target: {
//                 let parser = token::Parser::new(target);
//                 let first_seg = PathSegment::parse(&parser);
//                 Path::parse(&parser, first_seg)
//             },
//         }
//     }
//
//     pub fn target_buffer(&self) -> &Path { &self.target }
//
//     pub fn is_match(&self, state: &State) -> bool {
//         self.cond.as_ref().map(|c| c.run(state)).unwrap_or(true)
//     }
//     pub fn render(&self, target: &mut String, ctx: &Context) {
//         let ctx = ctx.items().iter()
//             // arbitrary, and i need a better system
//             .filter(|(_,v)| !matches!(v, Value::Map(_) | Value::Array(_) | Value::Regex(_)))
//             .map(|(path, value)| {
//                 let value = match &value {
//                     Value::Int(v) => Formattable::integer(v),
//                     Value::Uint(v) => Formattable::integer(v),
//                     Value::Float(v) => Formattable::float(v),
//                     Value::Bool(v) => Formattable::display(v),
//                     Value::Char(v) => Formattable::display(v),
//                     Value::Str(v) => Formattable::display(v),
//                     _ => unreachable!(),
//                 };
//                 (format!("{path}"), value)
//         })
//             .collect::<HashMap<_, _>>();
//         *target = interpolator::format(&self.fmt_str, &ctx).unwrap();
//     }
// }
//
// // @todo: add a `regex` value with a pre-compiled regex (faster matching?)
// #[derive(Debug, Clone)]
// pub enum Value {
//     Int(i64),
//     Uint(u64),
//     Float(f64),
//     Char(char),
//     Bool(bool),
//     Str(smol_str::SmolStr),
//     Array(Vec<Self>),
//     Map(HashMap<smol_str::SmolStr, Value>),
//     Regex(Regex),
// }
//
// impl Value {
//     fn as_bool(&self) -> Option<bool> {
//         if let Self::Bool(b) = self { Some(*b) } else { None }
//     }
//     pub fn as_array(&self) -> Option<Vec<Self>> {
//         if let Self::Array(a) = self { Some(a.clone()) } else { None }
//     }
//     fn as_map(&self) -> Option<HashMap<smol_str::SmolStr, Value>> {
//         if let Self::Map(m) = self { Some(m.clone()) } else { None }
//     }
//     pub fn as_string(&self) -> Option<smol_str::SmolStr> {
//         if let Self::Str(s) = self { Some(s.clone()) } else { None }
//     }
//     fn as_regex(&self) -> Option<Regex> {
//         if let Self::Regex(r) = self { Some(r.clone()) } else { None }
//     }
// }
//
// impl PartialEq for Value {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Self::Int(a), Self::Int(b)) => a == b,
//             (Self::Uint(a), Self::Uint(b)) => a == b,
//             (Self::Float(a), Self::Float(b)) => a == b,
//             (Self::Char(a), Self::Char(b)) => a == b,
//             (Self::Bool(a), Self::Bool(b)) => a == b,
//             (Self::Array(a), b) => a.iter().all(|a| a.eq(b)),
//             (Self::Str(a), Self::Str(b)) => a == b,
//             (Self::Regex(a), Self::Regex(b)) => a.as_str() == b.as_str(),
//             _ => false
//         }
//     }
// }
//
// impl PartialOrd for Value {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         match (self, other) {
//             (Self::Int(a), Self::Int(b)) => a.partial_cmp(b),
//             (Self::Uint(a), Self::Uint(b)) => a.partial_cmp(b),
//             (Self::Float(a), Self::Float(b)) => a.partial_cmp(b),
//             (Self::Char(a), Self::Char(b)) => a.partial_cmp(b),
//             (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
//             (Self::Array(a), Self::Array(b)) => a.iter().partial_cmp(b),
//             (Self::Str(a), Self::Str(b)) => a.partial_cmp(b),
//             (Self::Regex(a), Self::Regex(b)) => a.as_str().partial_cmp(b.as_str()),
//             _ => None
//         }
//     }
// }
//
// impl From<i64> for Value {
//     fn from(value: i64) -> Self {
//         Self::Int(value)
//     }
// }
// impl TryFrom<Value> for i64 {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Int(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// impl From<u64> for Value {
//     fn from(value: u64) -> Self {
//         Self::Uint(value)
//     }
// }
// impl TryFrom<Value> for u64 {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Uint(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// impl From<f64> for Value {
//     fn from(value: f64) -> Self {
//         Self::Float(value)
//     }
// }
// impl TryFrom<Value> for f64 {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Float(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// impl From<char> for Value {
//     fn from(value: char) -> Self {
//         Self::Char(value)
//     }
// }
// impl TryFrom<Value> for char {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Char(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// impl From<bool> for Value {
//     fn from(value: bool) -> Self {
//         Self::Bool(value)
//     }
// }
// impl TryFrom<Value> for bool {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Bool(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// // @todo: only allow singular data types in arrays
// impl From<Vec<Value>> for Value {
//     fn from(value: Vec<Value>) -> Self {
//         Self::Array(value)
//     }
// }
// impl TryFrom<Value> for Vec<Value> {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Array(v) = value { Ok(v) } else { Err(()) }
//     }
// }
//
// impl From<&str> for Value {
//     fn from(value: &str) -> Self {
//         Self::Str(smol_str::SmolStr::new(value))
//     }
// }
//
// impl From<String> for Value {
//     fn from(value: String) -> Self {
//         Self::Str(smol_str::SmolStr::new(value))
//     }
// }
// impl TryFrom<Value> for String {
//     type Error = ();
//     fn try_from(value: Value) -> Result<Self, Self::Error> {
//         if let Value::Str(v) = value { Ok(v.to_string()) } else { Err(()) }
//     }
// }
//
// #[derive(Debug)]
// pub struct Context {
//     data: BTreeMap<Path, Value>,
// }
//
// impl Context {
//     pub fn new() -> Self {
//         Self { data: BTreeMap::new() }
//     }
//     pub fn items(&self) -> &BTreeMap<Path, Value> {
//         &self.data
//     }
//
//     pub fn insert(&mut self, key: &str, val: impl Into<Value>) {
//         let parser = token::Parser::new(key);
//         let first_seg = PathSegment::parse(&parser);
//         let key = Path::parse(&parser, first_seg);
//
//         self.data.insert(key, val.into());
//     }
//     pub fn extend(&mut self, other: Context) {
//         for datum in other.data {
//             self.data.insert(datum.0, datum.1);
//         }
//     }
//
//     pub fn get<'g>(&'g self, key: &'g str) -> Option<&'g Value>
//     {
//         let parser = token::Parser::new(key);
//         let first_seg = PathSegment::parse(&parser);
//         let key = Path::parse(&parser, first_seg);
//
//         self.data.get(&key)
//     }
//     pub fn get_with_path<'g>(&'g self, key: &'g Path) -> Option<&'g Value> {
//         self.data.get(key)
//     }
//     pub fn get_with_path_mut<'g>(&'g mut self, key: &'g Path) -> Option<&'g mut Value> {
//         self.data.get_mut(key)
//     }
// }
//
// impl<T: Into<BTreeMap<Path, Value>>> From<T> for Context {
//     fn from(value: T) -> Self {
//         Self { data: value.into() }
//     }
// }
//
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// pub struct Path(Vec<PathSegment>);
// // deliberate code duplication to handle both condition and predicate entrypoints
// impl<'src> Path {
//     fn parse(parser: &token::Parser<'src>, first_seg: PathSegment) -> Self {
//         let mut segments = vec![first_seg];
//         while let (token::Token::Dot, _) = parser.peek() {
//             parser.consume(&token::Token::Dot);
//             segments.push(PathSegment::parse(parser));
//         }
//         Self(segments)
//     }
// }
//
// impl std::str::FromStr for Path {
//     type Err = ();
//     fn from_str(value: &str) -> Result<Self, Self::Err> {
//         let parser = token::Parser::new(value);
//         let first_seg = PathSegment::parse(&parser);
//         Ok(Path::parse(&parser, first_seg))
//     }
// }
//
// impl std::fmt::Display for Path {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match &self.0[0] {
//             PathSegment::Name(name) => f.write_str(name.as_str())?,
//             PathSegment::Index(idx) => write!(f, "{idx}")?,
//         }
//         for segment in &self.0[1..] {
//             match segment {
//                 PathSegment::Name(name) => write!(f, ".{name}")?,
//                 PathSegment::Index(idx) => write!(f, ".{idx}")?,
//             }
//         }
//         Ok(())
//     }
// }
//
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// pub enum PathSegment {
//     Name(smol_str::SmolStr),
//     Index(usize)
// }
//
// impl<'src> PathSegment {
//     fn parse(parser: &token::Parser<'src>) -> Self {
//         match parser.next().0 {
//             token::Token::Ident(ident) => {
//                 PathSegment::Name(smol_str::SmolStr::new(ident))
//             }
//             token::Token::Int(idx) if !idx.is_negative() => {
//                 PathSegment::Index(idx as usize)
//             }
//             t => panic!("unsupported token: {t:?}"),
//         }
//     }
// }
//
// pub trait Filterable {
//     // @todo: support more values (see `Value`)
//     // ^^ work these into the condition matching engine (parse integers, sets, etc)
//     fn get_context(&self) -> Context;
// }
