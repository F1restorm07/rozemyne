use secrecy::{ExposeSecret, SecretString};
use ureq::config::ConfigBuilder;
use ureq::http::uri::PathAndQuery;
use ureq::typestate::AgentScope;
use ureq::{Agent, Body};
use ureq::http::{Response, Uri};

use super::Source;

pub struct TMDBClient {
    agent: Agent,
    api_token: SecretString,
}

impl TMDBClient {
    pub fn new(config: ConfigBuilder<AgentScope>, api_token: SecretString) -> Self {
        Self {
            agent: config.build().new_agent(),
            api_token
        }
    }
}

impl<Req> Source<Req> for TMDBClient
where
    PathAndQuery: TryFrom<Req>,
    <PathAndQuery as TryFrom<Req>>::Error: Into<ureq::http::Error>,
{
    type Response = Response<Body>;
    type Error = ureq::Error;

    fn call(&mut self, req: Req) -> Result<Self::Response, Self::Error> {
        let uri = Uri::builder()
            .scheme("https")
            .authority("api.themoviedb.org")
            .path_and_query(req)
            .build().unwrap();
        self.agent.get(uri)
            .header("Authorization", &format!("Bearer {}", self.api_token.expose_secret()))
            .header("Accept", "application/json")
            .call()
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBMovie {
    adult: bool,
    backdrop_path: String,
    belongs_to_collection: TMDBCollection,
    budget: u32,
    genres: Vec<TMDBGenre>,
    homepage: String,
    id: TMDBId,
    imdb_id: String,
    origin_country: Vec<String>,
    original_language: String,
    original_title: String,
    overview: String,
    popularity: u32,
    poster_path: String,
    production_companies: Vec<TMDBProductionCo>,
    production_countries: Vec<TMDBCountry>,
    release_date: String,
    revenue: u32,
    runtime: u32,
    spoken_languages: Vec<TMDBLanguage>,
    status: String,
    tagline: String,
    title: String,
    video: bool,
    vote_average: f32,
    vote_count: i32
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBId(u32);

#[derive(Debug, serde::Deserialize)]
pub struct TMDBCollection {
    id: TMDBId,
    name: String,
    poster_path: String,
    backdrop_path: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBGenre {
    id: TMDBId,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBProductionCo {
    id: TMDBId,
    logo_path: String,
    name: String,
    origin_country: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBCountry {
    iso_3166_1: String,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TMDBLanguage {
    english_name: String,
    iso_639_1: String,
    name: String,
}
