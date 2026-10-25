use std::marker::PhantomData;

use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use enumset::{EnumSet, EnumSetType};
use secrecy::SecretString;
use uuid::Uuid;

use backend_schemas::db::FuzzyDate;

// an initial media request, agnostic of any data source
// this request will get dispersed into *every* data source that supports the queried media type
// ^^ doing the above is impossible with the current setup -> reformat data sources away from a trait into a component bundle
#[derive(Component)]
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
}

// every request has an id which ties it to the corresponding response
#[derive(Component)]
pub struct MediaDataRequest<Type> {
    req_id: Uuid,
    _pd: PhantomData<Type>,
}
impl<Type> MediaDataRequest<Type> {
    pub fn new() -> Self {
        Self { req_id: Uuid::now_v7(), _pd: PhantomData }
    }
    pub fn req_id(&self) -> Uuid { self.req_id }
}
#[derive(Component)]
// this id is the *same* as the id of the corresponding request
// this will be used when differentiating `MediaRequest`s belonging to the same client
pub struct MediaDataResponse<Type> {
    req_id: Uuid,
    _pd: PhantomData<(Type)>
}
impl<Type> MediaDataResponse<Type> {
    pub fn new(req_id: Uuid) -> Self {
        Self { req_id, _pd: PhantomData }
    }
    pub fn from_media_req(media_req: &MediaDataRequest<Type>) -> Self {
        Self { req_id: media_req.req_id, _pd: PhantomData }
    }
    pub fn req_id(&self) -> Uuid { self.req_id }
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
            client: HttpClient(reqwest::Client::new()),
            limiter: RateLimiter(governor::RateLimiter::direct(quota)),
        }
    }
}

// uniquely identifies media across data sources and requests
// how could i ensure that every media request associated with a particular media had the same media uuid?
#[derive(Debug, Clone, Component, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaRequestUuid(pub Uuid);

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
pub struct DataSourceBaseUrl(pub reqwest::Url);
#[derive(Component)]
pub struct DataSourceName(pub String);
#[derive(Component)]
pub struct SupportedMediaTypes(pub EnumSet<MediaType>);
#[derive(Component)]
pub struct SupportedQueryTypes(pub EnumSet<DataQueryType>);

#[derive(Component)]
pub struct HttpClient(pub reqwest::Client);
#[derive(Component)]
pub struct UdpClient(pub tokio::net::UdpSocket);
#[derive(Component)]
pub struct RateLimiter(pub governor::DefaultDirectRateLimiter);
// api key associated with a data source, required for all requests (accessed by children)
// optional for a given data source
#[derive(Component)]
pub struct DataSourceAPIKey(pub SecretString);

#[derive(Component)]
pub struct Media(pub MediaType);
// @todo: add image (for storing images associated with media, e.g. covers or banners)
#[derive(Debug, EnumSetType, Component)]
pub enum MediaType {
    Manga,
    Anime,
    Novel,
    Music,
}
#[derive(EnumSetType)]
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
pub struct MediaTitleQuery(pub String);
#[derive(Debug, Clone, Component)]
pub struct MediaSourceId(pub u32);
#[derive(Component)]
pub struct MediaSeasonQuery(pub u32);
#[derive(Component)]
pub struct MediaEpisodeQuery(pub u32);

// request + response types
// @todo: swap these out for `DataWuery`?
pub struct MediaSearchQuery;
pub struct MediaInfoQuery;
pub struct MediaItemQuery;
pub struct MediaCharacterQuery;
pub struct MediaStaffQuery;
pub struct MediaStudioQuery;
pub struct MediaRelationsQuery;
