#[derive(Debug)]
pub enum ClientError {
    Http(reqwest::Error),
    InvalidRequestType,
    InvalidResponseType,
    InvalidEventType,
    ParseError(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(err) => write!(f, "http error: {err}"),
            Self::InvalidRequestType => write!(f, "invalid request type"),
            Self::InvalidResponseType => write!(f, "invalid response type"),
            Self::InvalidEventType => write!(f, "invalid event type"),
            Self::ParseError(err) => write!(f, "parse error: {err}"),
        }
    }
}
impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            // Self::ParseError(e) => Some(e.downcast_ref()?),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct DeserializeError {
    inner: Box<dyn std::error::Error>,
}

impl DeserializeError {
    pub fn new(err: impl std::error::Error + 'static) -> Self {
        Self { inner: Box::new(err) }
    }
}

impl std::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}
impl std::error::Error for DeserializeError {}

#[derive(Debug)]
pub enum PingError {
    NoDataSourceAvailable(super::MediaType),
    Client(ClientError),
    InvalidResponseType,
}

impl std::fmt::Display for PingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDataSourceAvailable(m) =>
                f.write_fmt(format_args!("No data source available for the media type: {m:?}")),
            Self::Client(c) => f.write_fmt(format_args!("client error: {c}")),
            Self::InvalidResponseType => f.write_str("Invalid response type"),
        }
    }
}

impl std::error::Error for PingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoDataSourceAvailable(_) | Self::InvalidResponseType => None,
            Self::Client(c) => Some(c)
        }
    }
}
