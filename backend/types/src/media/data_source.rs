use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::event::Event;
use enumset::{EnumSet, EnumSetType};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::MediaType;

// an initial media request, agnostic of any data source
// this request will get dispersed into *every* data source that supports the queried media type
// ^^ doing the above is impossible with the current setup -> reformat data sources away from a trait into a component bundle
#[derive(Component, Clone)]
pub struct MediaRequest {
    pub id: Uuid,
    pub media_type: MediaType,
}
impl MediaRequest {
    pub fn new(media_type: MediaType) -> Self {
        Self {
            id: Uuid::now_v7(),
            media_type,
        }
    }
    pub fn with_id(id: Uuid, media_type: MediaType) -> Self {
        Self { id, media_type }
    }

    pub fn id(&self) -> Uuid { self.id }
}

#[derive(Event)]
pub struct NoInternalMedia; // no internal media found
// requesting data from *outside* the ECS system
#[derive(Component)]
pub struct MediaExternalRequest {
    id: Uuid,
    query_ty: DataQueryType,
    // slugs/ids of data sources
    // if this is None, then we hit *every* data source in the media type
    sources: Option<Vec<String>>,
    media_ty: MediaType,
}
impl MediaExternalRequest {
    pub fn new(query_ty: DataQueryType, media_ty: MediaType) -> Self {
        Self {
            id: Uuid::now_v7(),
            query_ty,
            media_ty,
            sources: None,
        }
    }
    pub fn from_internal_req(internal_req: MediaInternalRequest) -> Self {
        Self {
            id: internal_req.id,
            query_ty: internal_req.query_ty,
            media_ty: internal_req.media_ty,
            sources: None,
        }
    }

    pub fn with_sources(self, sources: Vec<String>) -> Self {
        Self { sources: Some(sources), ..self }
    }
    pub fn id(&self) -> Uuid { self.id }
    pub fn media_type(&self) -> MediaType { self.media_ty }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
    pub fn sources(&self) -> Option<&[String]> { self.sources.as_deref() }
}
#[derive(Component)]
pub struct MediaExternalResponse {
    id: Uuid,
    query_ty: DataQueryType,
    media_ty: MediaType,
}
impl MediaExternalResponse {
    pub fn from_request(req: MediaExternalRequest) -> Self {
        Self {
            id: req.id,
            query_ty: req.query_ty,
            media_ty: req.media_ty,
        }
    }
    pub fn id(&self) -> Uuid { self.id }
    pub fn media_type(&self) -> MediaType { self.media_ty }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
}

// requesting data from *within* the ECS system
#[derive(Component, Clone)]
pub struct MediaInternalRequest {
    id: Uuid,
    query_ty: DataQueryType,
    media_ty: MediaType,
}
impl MediaInternalRequest {
    pub fn new(query_ty: DataQueryType, media_ty: MediaType) -> Self {
        Self {
            id: Uuid::now_v7(),
            query_ty,
            media_ty
        }
    }
    pub fn id(&self) -> Uuid { self.id }
    pub fn media_type(&self) -> MediaType { self.media_ty }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
}
#[derive(Component)]
pub struct MediaInternalResponse {
    id: Uuid,
    query_ty: DataQueryType,
    media_ty: MediaType,
}
impl MediaInternalResponse {
    pub fn from_request(req: MediaInternalRequest) -> Self {
        Self {
            id: req.id,
            query_ty: req.query_ty,
            media_ty: req.media_ty,
        }
    }
    pub fn id(&self) -> Uuid { self.id }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
    pub fn media_type(&self) -> MediaType { self.media_ty }
}

// every request has an id which ties it to the corresponding response
#[derive(Component, Clone)]
pub struct DataSourceRequest {
    req_id: Uuid,
    query_ty: DataQueryType,
}
impl DataSourceRequest {
    pub fn new(query_ty: DataQueryType) -> Self {
        Self { req_id: Uuid::now_v7(), query_ty }
    }
    pub fn req_id(&self) -> Uuid { self.req_id }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
}
#[derive(Component)]
// this id is the *same* as the id of the corresponding request
// this will be used when differentiating `MediaRequest`s belonging to the same client
pub struct DataSourceResponse {
    req_id: Uuid,
    query_ty: DataQueryType,
}
impl DataSourceResponse {
    pub fn from_media_req(media_req: &DataSourceRequest) -> Self {
        Self { req_id: media_req.req_id, query_ty: media_req.query_ty }
    }
    pub fn req_id(&self) -> Uuid { self.req_id }
    pub fn query_type(&self) -> DataQueryType { self.query_ty }
}

#[derive(Bundle)]
pub struct MediaHttpClient<D: Component> {
    source: DataSource<D>,
    client: HttpClient,
    limiter: RateLimiter,
}
impl<D: Component> MediaHttpClient<D> {
    pub fn new(source: DataSource<D>, quota: governor::Quota) -> Self {
        Self {
            source,
            client: HttpClient(ureq::agent()),
            limiter: RateLimiter(governor::RateLimiter::direct(quota)),
        }
    }
}

// uniquely identifies media across data sources and requests
// how could i ensure that every media request associated with a particular media had the same media uuid?
#[derive(Debug, Clone, Component, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaRequestId(pub Uuid);

#[derive(Bundle)]
pub struct DataSource<D: Component> {
    source: D,
    name: DataSourceName,
    base_url: DataSourceBaseUrl,
    supported_media_types: SupportedMediaTypes,
    supported_query_types: SupportedQueryTypes
}
impl<D: Component> DataSource<D> {
    pub fn new(
        source: D,
        name: &str,
        base_url: &str,
        supported_media_types: EnumSet<MediaType>,
        supported_query_types: EnumSet<DataQueryType>,
    ) -> Self {
        Self {
            source,
            name: DataSourceName(name.to_string()),
            base_url: DataSourceBaseUrl(base_url.parse().unwrap()),
            supported_media_types: SupportedMediaTypes(supported_media_types),
            supported_query_types: SupportedQueryTypes(supported_query_types),
        }
    }
}

#[derive(Component)]
pub struct DataSourceBaseUrl(pub url::Url);
#[derive(Component)]
pub struct DataSourceName(pub String);
#[derive(Component)]
pub struct SupportedMediaTypes(pub EnumSet<MediaType>);
#[derive(Component)]
pub struct SupportedQueryTypes(pub EnumSet<DataQueryType>);

#[derive(Component)]
pub struct HttpClient(pub ureq::Agent);
#[derive(Component)]
pub struct UdpClient(pub std::net::UdpSocket);
#[derive(Component)]
pub struct RateLimiter(pub governor::DefaultDirectRateLimiter);
// api key associated with a data source, required for all requests (accessed by children)
// optional for a given data source
#[derive(Component)]
pub struct DataSourceAPIKey(pub SecretString);

#[derive(Component)]
pub struct Media(pub MediaType);
#[derive(Debug, EnumSetType, Serialize, Deserialize)]
pub enum DataQueryType {
    Search,
    Info,
    Character,
    Item, // e.g. episode, volume, etc
    Staff,
    Studio,
    Relations,
}

#[derive(Debug, Clone, Component)]
pub struct MediaSourceId(pub u32);
