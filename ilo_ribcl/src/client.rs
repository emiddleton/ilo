use async_recursion::async_recursion;
use async_trait::async_trait;
use ilo_console::ilo2::auth::Auth;
use native_tls::TlsConnector;
use native_tls::TlsStream;
use serde::{Deserialize, Serialize};
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::{
    fs::File,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    result::Result,
    str,
    vec::Vec,
};
use thiserror::Error;
use tracing::{event, instrument, Level};

use crate::{
    commands,
    types::{FwVersion, Version},
};

#[non_exhaustive]
#[derive(Error, Debug)] //, PartialEq)]
pub enum Error {
    /*
    #[error("io error: `{:#?}`", source)]
    Io {
        #[from]
        source: std::io::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    */
    #[error("TLS handshake failed: `{:#?}`", source)]
    TlsHandshake {
        #[from]
        source: native_tls::HandshakeError<std::net::TcpStream>,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /// Unable to write to tls stream
    #[error("tls write error: `{:#?}", source)]
    TlsWrite {
        #[from]
        source: std::io::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("tls error: `{source}`")]
    Tls {
        #[from]
        source: native_tls::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("https client builder error: `{:#?}`", source)]
    HttpsClientBuilder {
        source: reqwest::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("failed to make https connect")]
    HttpsConnection,

    #[error("https send error: `{:#?}`", source)]
    HttpsSend {
        #[from]
        source: reqwest::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /*
    #[error("error requesting page with reqwest `{:#?}`", source)]
    Reqwest {
        #[from]
        source: reqwest::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
    */
    #[error("invalid utf8 when parsing to str")]
    StrUtf8Error {
        #[from]
        source: str::Utf8Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("invalid utf8 when parsing to str")]
    StringUtf8Error {
        #[from]
        source: std::string::FromUtf8Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("command error: {source}")]
    Command {
        #[from]
        source: Box<commands::Error>,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    #[error("failed to determine node protocol version")]
    AutodetectFailed { source: Box<commands::Error> },

    #[error("unrecognized firmware version")]
    UnrecognizedFirmware(FwVersion),

    #[error("invalid endpoint json file: {source}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
}

impl str::FromStr for Version {
    type Err = &'static str;
    fn from_str(version: &str) -> Result<Self, Self::Err> {
        use Version::*;
        match version.to_lowercase().as_str() {
            "2" => Ok(Ilo2),
            "3" => Ok(Ilo3),
            "4" => Ok(Ilo4),
            _ => Err("Only version 2,3,4 or auto are supported value"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    auth: Auth,
    firmware: Option<FwVersion>,
    #[serde(skip)]
    client: Option<Box<dyn Client>>,
}

impl Node {
    pub async fn from_json(json: &str) -> Result<Self, Error> {
        match serde_json::from_str(json)? {
            Node {
                auth,
                firmware: None,
                ..
            } => Node::new(auth).await,
            Node {
                auth,
                firmware: Some(fw),
                ..
            } => Node::new_with_fw(auth, fw),
        }
    }

    pub async fn new(auth: Auth) -> Result<Self, Error> {
        Self::auto_detect(auth).await
    }

    pub fn new_with_fw(auth: Auth, fw_version: FwVersion) -> Result<Self, Error> {
        Ok(Self::new_with_fw_and_client(
            auth.clone(),
            fw_version.clone(),
            Self::client_from_auth_and_fw(&auth, &fw_version)?,
        ))
    }

    pub fn new_with_fw_and_client(
        auth: Auth,
        fw_version: FwVersion,
        client: Box<dyn Client>,
    ) -> Self {
        Self {
            auth,
            firmware: Some(fw_version),
            client: Some(client),
        }
    }

    #[async_recursion(?Send)]
    #[instrument(skip(self))]
    pub async fn send(&mut self, request: Vec<u8>) -> Result<String, Error> {
        loop {
            let firmware = self.firmware.clone();
            match self.client {
                Some(ref mut client) => return client.send(request).await,
                _ => match firmware {
                    Some(firmware) => {
                        let auth = self.auth.clone();
                        self.client = Some(Self::client_from_auth_and_fw(&auth, &firmware)?);
                    }
                    _ => {
                        let auth = self.auth.clone();
                        *self = Self::auto_detect(auth).await?;
                    }
                },
            }
        }
    }

    pub fn client_from_auth_and_fw(
        auth: &Auth,
        firmware: &FwVersion,
    ) -> Result<Box<dyn Client + Send>, Error> {
        use Version::*;
        match firmware {
            FwVersion {
                management_processor: Some(Ilo2),
                ..
            } => Ok(Box::new(TlsClient::new(auth.clone()))),
            FwVersion {
                management_processor: Some(Ilo3),
                ..
            }
            | FwVersion {
                management_processor: Some(Ilo4),
                ..
            } => Ok(Box::new(HttpsClient::new(auth.clone()))),
            _ => Err(Error::UnrecognizedFirmware(firmware.clone())),
        }
    }

    #[instrument]
    async fn auto_detect(auth: Auth) -> Result<Self, Error> {
        let mut node = Self {
            auth: auth.clone(),
            firmware: None,
            client: Some(Box::new(HttpsClient::new(auth.clone()))),
        };
        match node.get_fw_version().await {
            Ok(firmware) => {
                node.firmware = Some(firmware);
            }
            _ => {
                node.client = Some(Box::new(TlsClient::new(auth.clone())));
                node.firmware =
                    Some(
                        node.get_fw_version()
                            .await
                            .map_err(|err| Error::AutodetectFailed {
                                source: Box::new(err),
                            })?,
                    );
            }
        }
        Ok(node)
    }

    pub fn auth(&self) -> Auth {
        self.auth.clone()
    }

    pub fn firmware(&self) -> Option<FwVersion> {
        self.firmware.clone()
    }
}

#[async_trait]
pub trait Client: std::fmt::Debug + Send {
    async fn send(&mut self, request: Vec<u8>) -> Result<String, Error>;
}

#[derive(Debug)]
pub struct TlsClient {
    pub auth: Auth,
}

impl TlsClient {
    pub fn new(auth: Auth) -> Self {
        Self { auth }
    }

    #[instrument(skip(self))]
    pub fn tls_stream(&mut self) -> Result<TlsStream<TcpStream>, Error> {
        let mut builder = TlsConnector::builder();
        builder.danger_accept_invalid_certs(true);
        let connector = builder.build()?;
        let url = format!("{}:443", self.auth.hostname);
        let stream = TcpStream::connect(&url)?;
        let tls_stream = connector.connect(&self.auth.hostname, stream)?;
        Ok(tls_stream)
    }
}

#[async_trait]
impl Client for TlsClient {
    #[instrument(skip(self))]
    async fn send(&mut self, request: Vec<u8>) -> Result<String, Error> {
        let mut stream = self.tls_stream()?;
        stream
            .write_all(&request)
            .map_err(|source| Error::TlsWrite { source })?;
        let mut response = vec![];
        event!(
            Level::DEBUG,
            request = String::from_utf8_lossy(&request).as_ref()
        );
        stream.read_to_end(&mut response)?;
        let response = String::from_utf8(response)?;
        event!(Level::DEBUG, ?response);
        Ok(response)
    }
}

#[derive(Debug)]
pub struct HttpsClient {
    pub auth: Auth,
    http_client: Option<reqwest::Client>,
}

impl HttpsClient {
    pub fn new(auth: Auth) -> Self {
        Self {
            auth,
            http_client: None,
        }
    }

    #[instrument(skip(self))]
    pub fn http_client(&mut self) -> Result<&mut Option<reqwest::Client>, Error> {
        if self.http_client.is_none() {
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .use_native_tls()
                .cookie_store(true)
                .build()
                .map_err(|source| Error::HttpsClientBuilder { source })?;
            self.http_client = Some(client);
        }
        Ok(&mut self.http_client)
    }
}

#[async_trait]
impl Client for HttpsClient {
    async fn send(&mut self, request: Vec<u8>) -> Result<String, Error> {
        let url = format!("https://{}:443/ribcl", &self.auth.hostname);
        if let Some(client) = self.http_client()? {
            event!(
                Level::DEBUG,
                request = String::from_utf8_lossy(&request).as_ref()
            );
            let response = client.post(&url).body(request).send().await?.text().await?;
            event!(Level::DEBUG, ?response);
            Ok(response)
        } else {
            Err(Error::HttpsConnection)
        }
    }
}

#[derive(Debug)]
pub struct ProxyClient {
    auth: Auth,
    firmware: FwVersion,
    client: Box<dyn Client + Send>,
}

impl ProxyClient {
    pub fn new(auth: Auth, firmware: FwVersion, client: Box<dyn Client + Send>) -> Self {
        Self {
            auth,
            firmware,
            client,
        }
    }

    fn hash_request(&self, request: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::new();
        hasher.write(request);
        hasher.finish()
    }
}

#[async_trait]
impl Client for ProxyClient {
    async fn send(&mut self, request: Vec<u8>) -> Result<String, Error> {
        let hash_id = self.hash_request(&request);
        let request_filename = format!("{}-{}-request.xml", self.auth.hostname, hash_id);
        let response_filename = format!("{}-{}-response.xml", self.auth.hostname, hash_id);
        if Path::new(&request_filename).is_file() && Path::new(&response_filename).is_file() {
            event!(
                Level::DEBUG,
                request = String::from_utf8_lossy(&request).as_ref()
            );
            let mut f = File::open(&response_filename)?;
            let mut response = String::new();
            f.read_to_string(&mut response)?;
            event!(Level::DEBUG, ?response);
            Ok(response)
        } else {
            File::create(&request_filename)?.write_all(&request)?;
            let response = self.client.send(request).await?;
            File::create(&response_filename)?.write_all(response.as_bytes())?;
            Ok(response)
        }
    }
}
