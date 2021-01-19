extern crate base64;
extern crate reqwest;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{default::Default, time::Duration};
use thiserror::Error;
use tokio::time::delay_for;
use tracing::{event, instrument, Level};

use crate::{find, ilo2::session, ilo2::session::Parameters};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Auth {
    #[serde(skip, default = "client_default")]
    client: Option<reqwest::Client>,
    #[serde(skip)]
    parameters: Option<Parameters>,
    #[serde(skip)]
    config_file: String,
    pub hostname: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    session_index: String,
    #[serde(default)]
    session_key: String,
    #[serde(default)]
    cookie: String,
}

fn client_default() -> Option<reqwest::Client> {
    None
}

impl Default for Auth {
    fn default() -> Self {
        Auth {
            client: client_default(),
            parameters: None,
            config_file: Default::default(),
            hostname: Default::default(),
            username: Default::default(),
            password: Default::default(),
            session_index: Default::default(),
            session_key: Default::default(),
            cookie: Default::default(),
        }
    }
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("error requesting page from ilo node: `{0}`")]
    Reqwest(#[from] reqwest::Error),
    #[error("error searching for value in web page: `{0}`")]
    Find(#[from] find::Error),
    #[error("error ocurred loading session config: `{0}`")]
    SessionParameter(#[from] session::ParameterError),
    #[error("error occurred loading session: `{0}`")]
    Session(#[from] session::Error),
    #[error("failed to connect to node")]
    ConnectionFailed,
    #[error("client not loaded")]
    ClientNotLoaded,
}

impl Auth {
    fn client(&mut self) -> &mut Option<reqwest::Client> {
        if self.client.is_none() {
            // set default headers
            let mut headers = HeaderMap::new();
            {
                use reqwest::header::{HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CONNECTION};
                headers.insert(
                    ACCEPT,
                    HeaderValue::from_static("text/html, application/xhtml+xml, image/jxr, */*"),
                );
                headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("un-US"));
                headers.insert(CONNECTION, HeaderValue::from_static("Keep-Alive"));
            }

            // setup http client
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .use_native_tls()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; WOW64; Trident/7.0; rv:11.0) like Gecko")
                .default_headers(headers)
                .cookie_store(true)
                .build()
                .unwrap();
            self.client = Some(client);
        }
        &mut self.client
    }

    #[instrument(skip(self))]
    pub async fn authenticate(&mut self) -> Result<(), Error> {
        if self.cookie.is_empty() {
            event!(Level::INFO, "no cookie found generating cookie");
            self.generate_cookie().await?;
        } else {
            // get session key and index
            event!(Level::INFO, "cookie found testing validity");
            let url = format!("https://{}/ie_index.htm", &self.hostname);
            let body = if let Some(client) = &mut self.client() {
                client
                    .get(&url)
                    .header(reqwest::header::COOKIE, &self.cookie)
                    .send()
                    .await?
                    .text()
                    .await?
            } else {
                return Err(Error::ClientNotLoaded);
            };
            if find::has(r#"Login Delay"#, &body)?
                || find::has(r#"Integrated Lights-Out 2 Login"#, &body)?
            {
                event!(Level::INFO, "generating new cookie");
                self.generate_cookie().await?;
            } else {
                event!(Level::INFO, "cookie validated");
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn parameters(&mut self) -> Result<Parameters, Error> {
        if self.parameters.is_none() {
            self.authenticate().await?;
            self.load_parameters().await?;
        }
        self.parameters.clone().ok_or(Error::ConnectionFailed)
    }

    #[instrument]
    async fn load_parameters(&mut self) -> Result<(), Error> {
        let mut body;
        let url = format!("https://{}/drc2fram.htm?restart=1", &self.hostname);
        loop {
            body = if let Some(client) = &mut self.client() {
                client
                    .get(&url)
                    .header(reqwest::header::COOKIE, &self.cookie)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap()
            } else {
                return Err(Error::ClientNotLoaded);
            };
            event!(Level::TRACE, ?body);
            if find::has(
                r#"The Remote Console is unavailable, it is already in use by a different client."#,
                &body,
            )? {
                event!(Level::WARN, "already connected");
                delay_for(Duration::from_millis(3000)).await;
            } else {
                break;
            }
        }
        self.parameters = Some(Parameters::try_from(&body, self.hostname.clone())?);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn generate_cookie(&mut self) -> Result<(), Error> {
        // get session key and index
        let url = format!("https://{}/index.htm", &self.hostname);
        loop {
            let body = if let Some(client) = &mut self.client() {
                client.get(&url).send().await?.text().await?
            } else {
                return Err(Error::ClientNotLoaded);
            };
            let session_key = find::by_regex(r#"var sessionkey="([^"]+)";"#, &body)?.to_string();
            if session_key != "NONEAVAILABLE" {
                self.session_key = session_key;
                self.session_index =
                    find::by_regex(r#"var sessionindex="([^"]+)";"#, &body)?.to_string();
                break;
            }
            event!(
                Level::WARN,
                ?session_key,
                "no session keys available delaying before retrying"
            );
            delay_for(Duration::from_millis(1000)).await;
        }
        event!(Level::DEBUG, ?self.session_key, ?self.session_index);

        // step 2
        let login_url = format!("https://{}/index.htm", self.hostname);
        let res = if let Some(client) = &mut self.client() {
            client
                .get(&login_url)
                .header(
                    reqwest::header::COOKIE,
                    format!(
                        "hp-iLO-Login={}:{}:{}:{}",
                        self.session_index,
                        base64::encode(self.username.as_bytes()),
                        base64::encode(self.password.as_bytes()),
                        self.session_key
                    ),
                )
                .send()
                .await
                .unwrap()
        } else {
            return Err(Error::ClientNotLoaded);
        };
        for cookie in res.headers().get_all("Set-Cookie").iter() {
            event!(Level::TRACE, ?cookie);
        }
        let session_cookie = res
            .headers()
            .get_all("Set-Cookie")
            .iter()
            .map(|c| c.to_str().unwrap_or(""))
            .find(|c| c.starts_with("hp-iLO-Session="))
            .unwrap();
        event!(Level::DEBUG, session_cookie);
        self.cookie = String::from(session_cookie);

        Ok(())
    }
}
