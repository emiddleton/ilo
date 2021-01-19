use regex::Regex;
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("invalid regex used to find: {0}")]
    Regexp(#[from] regex::Error),
    #[error("value could not be found")]
    Missing,
}

pub fn by_regex<'a>(rxp: &str, body: &'a str) -> Result<&'a str, Error> {
    Ok(Regex::new(rxp)
        .map_err(Error::Regexp)?
        .captures(body)
        .ok_or(Error::Missing)?
        .get(1)
        .ok_or(Error::Missing)?
        .as_str())
}

pub fn has(rxp: &str, body: &str) -> Result<bool, Error> {
    Ok(Regex::new(rxp).map_err(Error::Regexp)?.is_match(&body))
}
