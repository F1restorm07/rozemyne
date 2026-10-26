use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::iter::Peekable;

use backend_types::renamer::RenamerScript;
// use backend_types::renamer::{Context, FormatPattern, Pattern, PatternMatch, RenamerScript, State, Value};
// use backend_types::renamer::filter::{StoredFilter, StoredFunction, StoredTest};
use bevy_asset::AssetServer;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Commands, Res};
use interpolator::Formattable;
use logos::{Logos, Span, SpannedIter};

use crate::configuration::Settings;

// pub mod filters;

pub fn load_renamer_script(
    mut cmds: Commands,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    // load script into asset server
    let script = asset_server.load(settings.renamer_script().to_owned());
    // load renamer modules
    // how can i dynamically insert the renamed file as input to the script?
    let mut lua_ctx = mlua::Lua::new();

    // lua_ctx.register_module("", value);

    cmds.insert_resource(RenamerScript { script, context: lua_ctx });
}

// @todo: how do i build this into a tree/graph? <- see [matcher_rs]
// how do i nest these with combinators or a trie structure? (ex. a not combinator which matches when the matcher does not)
// @todo: implement rete algorithm on top of `Pattern` -> the rete algorithm requires that the nodes "be" the pattern in a sense
// #[derive(Resource)]
// pub struct PatternEngine {
//     patterns: Vec<Pattern>,
//     // should there be a final formatting pattern? (would this be optional?)
//     format_patterns: Vec<FormatPattern>,
//     filters: HashMap<String, StoredFilter>,
//     tests: HashMap<String, StoredTest>,
//     functions: HashMap<String, StoredFunction>,
//     default_formatter: StoredFilter,
// }
//
// // @todo: try and replicate the `regex::Regex` API with matches
// impl PatternEngine {
//     pub fn new() -> Self {
//         Self {
//             patterns: Vec::new(),
//             format_patterns: Vec::new(),
//             filters: [
//                 ("lower".to_string(), StoredFilter::new(filters::lower)),
//                 ("upper".to_string(), StoredFilter::new(filters::upper)),
//                 ("trim_whitespace".to_string(), StoredFilter::new(filters::trim_whitespace)),
//                 ("trim".to_string(), StoredFilter::new(filters::trim)),
//                 ("prepend".to_string(), StoredFilter::new(filters::prepend)),
//                 ("append".to_string(), StoredFilter::new(filters::append)),
//                 ("push".to_string(), StoredFilter::new(filters::push)),
//                 ("insert".to_string(), StoredFilter::new(filters::insert)),
//             ].into_iter().collect(),
//             tests: [
//                 ("int".to_string(), StoredTest::new(filters::is_int)),
//                 ("uint".to_string(), StoredTest::new(filters::is_unsigned_int)),
//                 ("float".to_string(), StoredTest::new(filters::is_float)),
//                 ("char".to_string(), StoredTest::new(filters::is_char)),
//                 ("bool".to_string(), StoredTest::new(filters::is_bool)),
//                 ("odd".to_string(), StoredTest::new(filters::is_odd)),
//                 ("any".to_string(), StoredTest::new(filters::any)),
//                 ("all".to_string(), StoredTest::new(filters::all)),
//             ].into_iter().collect(),
//             functions: [
//                 ("exists".to_string(), StoredFunction::new(filters::exists)),
//             ].into_iter().collect(),
//             default_formatter: StoredFilter::new(|val: Value, _: backend_types::renamer::filter::Kwargs, _: &Context| -> String {
//                 match val {
//                     Value::Int(v) => format!("{v}"),
//                     Value::Uint(v) => format!("{v}"),
//                     Value::Float(v) => format!("{v}"),
//                     Value::Bool(v) => format!("{v}"),
//                     Value::Char(v) => format!("{v}"),
//                     Value::Str(v) => v.to_string(),
//                     v => panic!("unformattable type: {v:?}"),
//                 }
//             })
//         }
//     }
//     pub fn state<'ctx>(&self, ctx: &'ctx mut Context) -> State<'_, 'ctx> {
//         State {
//             context: ctx,
//             filters: &self.filters,
//             tests: &self.tests,
//             functions: &self.functions,
//             default_formatter: &self.default_formatter,
//
//         }
//     }
//
//     pub fn add_pattern(&mut self, cond: Option<&str>, pred: &str, target_buffer: Option<&str>)
//         { self.patterns.push(Pattern::new(cond, pred, target_buffer)); }
//     pub fn add_format_pattern(&mut self, cond: Option<&str>, patt: String, target_buf: &str)
//         { self.format_patterns.push(FormatPattern::new(cond, patt, target_buf)); }
//
//     pub fn get_matches(&self, state: &State) -> Vec<PatternMatch<'_>> {
//         self.patterns.iter().filter(|p| p.is_match(state))
//             .map(PatternMatch::new).collect()
//     }
//     pub fn get_format_matches(&self, state: &State) -> Vec<&FormatPattern> {
//         self.format_patterns.iter().filter(|p| p.is_match(state))
//             .collect()
//     }
//
//     // how do i render to multiple target buffers?
//     pub fn render(&self, ctx: &mut Context) {
//         let state = self.state(ctx);
//
//         let matches = self.get_matches(&state);
//         let fmt_matches = self.get_format_matches(&state);
//         let mut global_buffer = String::new();
//
//         // how can i inject the targets into the predicates?
//         // that way, i can do away with all the target buffer stuff
//         //
//         // in other words, how can i modify the state within the predicate?
//         // one way would be enforcing a path as the first expression, then progressively modifying it
//         let mut target_buffers = BTreeMap::new();
//         for (path, value) in state.context.items() {
//             if self.patterns.iter().any(|p| p.target_buffer().as_ref() == Some(path))
//                 || self.format_patterns.iter().any(|p| p.target_buffer() == path)
//             {
//                 target_buffers.insert(path.clone(), value.as_string().unwrap().to_string());
//             }
//         }
//
//         for mat in matches {
//             let target_buf = mat.target_buffer().as_ref()
//                 .map(|t| target_buffers.get_mut(t).unwrap())
//                 .unwrap_or(&mut global_buffer);
//
//             mat.render(target_buf, &state);
//         }
//         for mat in fmt_matches {
//             let target_buf = target_buffers.get_mut(mat.target_buffer()).unwrap();
//             mat.render(target_buf, state.context);
//         }
//
//         // pipe the altered items back into the context (for actual renaming)
//         for (path, val) in target_buffers {
//             let item = ctx.get_with_path_mut(&path).unwrap();
//             *item = Value::Str(val.into());
//         }
//     }
// }
//
// pub struct Filename {
//     season: String,
//     season_episode: String,
//     absolute_episode: String,
//     year: String,
//     media_type: String,
//     media_name: String,
//     group: String,
//     quality: String,
//     rip_source: String,
//     dub_language: String,
//     sub_language: String,
//     video_codec: String,
// }
//
// // taken from `whereexpr`
// pub trait Attributes {
//     fn get_value(&self, id: usize) -> Option<Value>;
//     fn get_index(field: &str) -> Option<usize>;
//     fn get_field_name(id: usize) -> Option<&'static str>;
//     fn type_id() -> std::any::TypeId;
//     fn type_name() -> &'static str;
// }
//
// pub enum RenamerError {
//     UnknownFieldAttr,
// }
