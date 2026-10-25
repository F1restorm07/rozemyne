use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use backend_types::media::MediaType;
use backend_types::media::data_source::DataQueryType;
use bevy_ecs::resource::Resource;
use enumset::EnumSet;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::files::renamer::PatternEngine;

// @todo: add a general settings table
#[derive(Debug, Resource, Deserialize)]
pub struct Settings {
    #[serde(rename = "media_folder")]
    media_folders: Vec<MediaFolder>,
    archive: ArchiveSettings,
    #[serde(rename = "data_source")]
    data_sources: Vec<DataSource>,
    pattern_file: PathBuf,
    #[serde(with = "humantime_serde")]
    data_scanning_freq: Duration,
    hash_buffer_size: String,
}

impl Settings {
    pub fn archive_path(&self) -> &Path { &self.archive.path }
    pub fn media_folders(&self) -> &[MediaFolder] { &self.media_folders }
    pub fn pattern_file(&self) -> &Path { &self.pattern_file }
    pub fn hash_buffer_size(&self) -> u64
        { parse_size::parse_size(&self.hash_buffer_size).unwrap() }
    pub fn data_scanning_frequency(&self) -> Duration { self.data_scanning_freq }
    pub fn archive_frequency(&self) -> Duration { self.archive.frequency }
}

#[derive(Debug, Deserialize)]
pub struct MediaFolder {
    id: String,
    import_path: PathBuf,
    destination_path: PathBuf,
    media_type: MediaType,
}

impl MediaFolder {
    pub fn id(&self) -> &str { &self.id }
    pub fn import_path(&self) -> &Path { &self.import_path }
    pub fn destination_path(&self) -> &Path { &self.destination_path }
    pub fn media_type(&self) -> MediaType { self.media_type }
}

#[derive(Debug, Deserialize)]
pub struct ArchiveSettings {
    path: PathBuf,
    #[serde(with = "humantime_serde")]
    frequency: Duration,
}

#[derive(Debug, Deserialize)]
pub struct DataSource {
    name: String,
    media_types: Vec<MediaType>,
    auth: Option<AuthInfo>,
    supported_queries: Vec<DataQueryType>
}

impl DataSource {
    pub fn name(&self) -> &str { &self.name }
    pub fn media_types(&self) -> enumset::EnumSet<MediaType> { self.media_types.iter().collect() }
    pub fn supported_queries(&self) -> EnumSet<DataQueryType> { self.supported_queries.iter().collect() }
}

// @todo: flesh out this api
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AuthInfo {
    APIKey(SecretString),
}

// @todo: move *all* the pattern related settings here (i.e. formatting string, new functions, etc)
#[derive(Debug, Deserialize)]
pub struct Patterns {
    #[serde(rename = "pattern")]
    patterns: Vec<NamingPattern>,
    #[serde(rename = "format_pattern")]
    format_patterns: Vec<FormattingPattern>,
}

// @todo: pull this into a more specific table (have a broader renamer table with `format_pattern` etc)
#[derive(Debug, Deserialize)]
pub struct NamingPattern {
    condition: Option<String>,
    predicate: String,
    target: String,
}
#[derive(Debug, Deserialize)]
pub struct FormattingPattern {
    condition: Option<String>,
    pattern: String,
    target: String,
}

pub struct ConfigPlugin {
    path: PathBuf,
}

impl ConfigPlugin {
    pub fn new(path: impl AsRef<std::path::Path>) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }
}

impl bevy_app::Plugin for ConfigPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // let path = app.world().get_resource::<crate::cli::Args>().unwrap();
        // @todo: read path from cli, or fallback to default
        let file = fs::read(&self.path).unwrap();
        let mut config: Settings = toml::from_slice(&file).unwrap();
        config.pattern_file = if config.pattern_file.is_relative() {
            self.path.with_file_name(config.pattern_file)
        } else {
            config.pattern_file
        };

        let patterns_file = fs::read(&config.pattern_file).unwrap();
        let patterns: Patterns = toml::from_slice(&patterns_file).unwrap();

        let mut engine = PatternEngine::new();
        for pat in patterns.patterns {
            if !pat.predicate.is_empty() {
                engine.add_pattern(pat.condition.as_deref(), &pat.predicate, Some(&pat.target));
            }
        }
        for pat in patterns.format_patterns {
            if !pat.pattern.is_empty() {
                engine.add_format_pattern(pat.condition.as_deref(), pat.pattern, &pat.target);
            }
        }

        app.insert_resource(config);
        app.insert_resource(engine);
    }
}
