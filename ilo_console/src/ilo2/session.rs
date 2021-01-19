use regex::Regex;
use serde::{Deserialize, Serialize};
use std::clone::Clone;
use thiserror::Error;
use tracing::{event, instrument, Level};

use crate::find;

const SESSION_TIMEOUT_DEFAULT: u32 = 900;

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Parameters {
    ipaddr: String,
    info0: String,
    info1: String,
    info6: String,
    info7: String,
    infoa: String,
    infob: String,
    infoc: String,
    infod: String,
    infom: String,
    infomm: String,
    infon: String,
    infoo: String,
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum ParameterError {
    #[error("could not be found")]
    NotFound(#[from] find::Error),
    #[error("did not contain a value")]
    Empty(String),
}

impl Parameters {
    pub fn try_from(body: &str, host: String) -> Result<Parameters, ParameterError> {
        Ok(Parameters {
            ipaddr: host,
            info0: Self::find("0", &body, true)?,
            info1: Self::find("1", body, true)?,
            info6: Self::find("6", body, true)?,
            info7: Self::find("7", body, false)?,
            infoa: Self::find("a", body, true)?,
            infob: Self::find("b", body, true)?,
            infoc: Self::find("c", body, true)?,
            infod: Self::find("d", body, true)?,
            infom: Self::find("m", body, false)?,
            infomm: Self::find("mm", body, false)?,
            infon: Self::find("n", body, false)?,
            infoo: Self::find("o", body, true).unwrap_or_default(),
        })
    }

    #[instrument]
    fn find<'a>(sym: &str, body: &'a str, wrapped: bool) -> Result<String, ParameterError> {
        let rxp = if wrapped {
            format!(r#"info{}="([^"]+)";"#, sym)
        } else {
            format!(r#"info{}=([^;]+);"#, sym)
        };
        event!(Level::TRACE, "finding info{}", sym);
        let value = find::by_regex(&rxp, body)?.trim().to_string();
        if value.is_empty() {
            return Err(ParameterError::Empty(format!("info{}", sym)));
        }
        Ok(value)
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Session {
    pub host: String,
    pub login: String,
    pub port: u32,
    pub encryption_enabled: bool,
    pub encrypt_key: Vec<u8>,
    pub key_index: u32,
    pub decrypt_key: Vec<u8>,
    pub session_timeout: u32,
    pub launch_terminal_services: bool,
    pub ts_param: u32,
    pub terminal_services_port: u32,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("first part of Compaq RIB Login is missing: `{0}`")]
    CompaqRIBLoginMissingFirst(String),
    #[error("second part of Compaq RIB Login is missing: `{0}`")]
    CompaqRIBLoginMissingLast(String),
    #[error("param {param:?} value {value:?} does not appear to be base64 encoded")]
    InvalidBase64Value {
        param: String,
        value: String,
        source: base64::DecodeError,
    },
    #[error("param {param:?} value {value:?} does not appear to be base64 encoded")]
    InvalidHexValue {
        param: String,
        value: String,
        source: hex::FromHexError,
    },
    #[error("login can't be converted to utf8 string")]
    InvalidUtf8Value {
        param: String,
        value: String,
        source: std::string::FromUtf8Error,
    },
    #[error("param {param:?} value {value:?} is invalid")]
    InvalidIntValue {
        param: String,
        value: String,
        source: std::num::ParseIntError,
    },
}

impl Session {
    #[instrument]
    fn parse_login(info0: &str) -> Result<String, Error> {
        let rxp = Regex::new(r#"Compaq-RIB-Login=(.{56})(.{32})"#).unwrap();
        use Error::*;
        let login = match rxp.captures(info0) {
            Some(caps) => format!(
                "\x1B[!{}\r{}\r",
                caps.get(1)
                    .ok_or_else(|| CompaqRIBLoginMissingFirst(info0.to_string()))?
                    .as_str(),
                caps.get(2)
                    .ok_or_else(|| CompaqRIBLoginMissingLast(info0.to_string()))?
                    .as_str()
            ),
            None => {
                let login_bytes = base64::decode(info0).map_err(|err| InvalidBase64Value {
                    param: String::from("login"),
                    value: info0.to_string(),
                    source: err,
                })?;
                String::from_utf8(login_bytes)
                    .map(|c| format!("{}\r", c))
                    .map_err(|err| InvalidUtf8Value {
                        param: String::from("login"),
                        value: info0.to_string(),
                        source: err,
                    })?
            }
        };
        Ok(login)
    }

    #[instrument(skip(parameters))]
    pub fn try_from(parameters: Parameters) -> Result<Session, Error> {
        use Error::*;
        let login = Self::parse_login(&parameters.info0).map(|login| {
            if !parameters.info1.is_empty() {
                format!("\u{1b}[7\u{1b}[9\u{1b}[4{}", login)
            } else {
                format!("\u{1b}[7\u{1b}[9{}", login)
            }
        })?;

        let port = parameters.info6.parse::<u32>().unwrap_or(23);
        let session_timeout = parameters
            .info7
            .parse::<u32>()
            .map_err(|err| {
                event!(
                    Level::WARN,
                    ?err,
                    from = "info7",
                    default = SESSION_TIMEOUT_DEFAULT,
                    "session timeout value can't be parsed using",
                )
            })
            .map(|j| j * 60)
            .unwrap_or(SESSION_TIMEOUT_DEFAULT);

        let encryption_enabled = parameters
            .infoa
            .parse::<u32>()
            .map_err(|err| {
                event!(
                    Level::WARN,
                    ?err,
                    from = "infoa",
                    default = false,
                    "encryption enabled value can't be parsed using"
                )
            })
            .map(|i| i == 1)
            .unwrap_or(false);

        let mut decrypt_key: Vec<u8> = Vec::new();
        let mut encrypt_key: Vec<u8> = Vec::new();
        let mut key_index: u32 = 0;

        if encryption_enabled {
            decrypt_key = hex::decode(&parameters.infob).map_err(|err| InvalidHexValue {
                param: String::from("decrypt_key"),
                value: parameters.infoc.clone(),
                source: err,
            })?;

            encrypt_key = hex::decode(&parameters.infoc).map_err(|err| InvalidHexValue {
                param: String::from("encrypt_key"),
                value: parameters.infoc.clone(),
                source: err,
            })?;

            key_index = parameters
                .infod
                .parse::<i32>()
                .map(|i| i as u32)
                .map_err(|err| InvalidIntValue {
                    param: String::from("key_index"),
                    value: parameters.infod.clone(),
                    source: err,
                })?;
        }

        let infon = parameters
            .infon
            .parse::<u32>()
            .map_err(|err| {
                event!(
                    Level::WARN,
                    ?err,
                    default = 0,
                    "Terminal Service Params can't be parsed using"
                )
            })
            .unwrap_or(0);
        let mut ts_param = infon & 0xFF00;
        ts_param &= 0xFF;
        let launch_terminal_services;
        match infon & 0xff {
            0 => {
                launch_terminal_services = false;
                ts_param |= 0x1
            }
            1 => launch_terminal_services = false,
            _ => {
                launch_terminal_services = true;
                ts_param |= 0x1
            }
        }

        let terminal_services_port = parameters
            .infoo
            .parse::<u32>()
            .map_err(|err| {
                event!(
                    Level::WARN,
                    ?err,
                    from = "infoo",
                    default = 3389,
                    "terminal service port can't be parsed using"
                )
            })
            .unwrap_or(3389);

        let host = parameters.ipaddr;

        Ok(Session {
            login,
            host,
            port,
            encryption_enabled,
            encrypt_key,
            key_index,
            decrypt_key,
            session_timeout,
            launch_terminal_services,
            ts_param,
            terminal_services_port,
        })
    }
}
