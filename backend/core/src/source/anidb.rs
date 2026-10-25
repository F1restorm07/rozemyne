use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use secrecy::{ExposeSecret, SecretString};

use super::Source;

// aniDB api steps
// 1. grab config with username, password, client name and version, and port
// 2. setup rate limiter
// 3. authenticate with aniDB, and retrieve session string
// 4. call aniDB api with command (via Source)

pub struct AniDBClient {
    config: AniDBConfig,
    socket: UdpSocket,
    authenticated: bool,
    encrypted: bool,
    session_key: Option<String>,
}

impl AniDBClient {
    pub fn new(config: AniDBConfig) -> Self {
        let socket = UdpSocket::bind(
            SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            config.port
        )).unwrap();
        Self {
            config,
            socket,
            authenticated: false,
            encrypted: false,
            session_key: None,
        }
    }

    pub fn auth(&mut self) {
        let auth_cmd = AniDBCommand::new("auth", [
            ("user", self.config.username.as_str()),
            ("pass", self.config.password.expose_secret()),
            ("protover", "3"),
            ("client", &self.config.client_name),
            ("clientver", &format!("{}", self.config.client_ver)),
            ("enc", "UTF8"),
        ]);
    }
}

impl Source<AniDBCommand<'_>> for AniDBClient {
    type Response = ();
    type Error = ();

    fn call(&mut self, req: AniDBCommand<'_>) -> Result<Self::Response, Self::Error> {
        // @todo: setup an idle timer which automatically logs the client out
        // then, if another command comes in later, and the client has logged out, reauth and continue
        if self.authenticated {

        } else {
            self.auth();
        }

        Ok(())
    }
}

pub struct AniDBConfig {
    // only used for encrypton, thus it's optional
    api_key: Option<SecretString>,
    username: String,
    password: SecretString,
    port: u16,
    client_name: String,
    client_ver: i32,
}

impl AniDBConfig {
    pub fn new(username: String, password: SecretString, client_name: String, client_ver: i32, port: u16) -> Self {
        Self {
            username,
            password,
            client_name,
            client_ver,
            api_key: None,
            port,
        }
    }
    pub fn port(self, port: u16) -> Self {
        Self { port, ..self }
    }
    pub fn api_key(self, api_key: SecretString) -> Self {
        Self { api_key: Some(api_key), ..self }
    }
}

pub struct AniDBCommand<'cmd> {
    name: &'cmd str,
    map: HashMap<&'cmd str, &'cmd str>
}

impl<'cmd> AniDBCommand<'cmd> {
    pub fn new(name: &'cmd str, map: impl Into<HashMap<&'cmd str, &'cmd str>>) -> Self {
        Self { name, map: map.into() }
    }
}
