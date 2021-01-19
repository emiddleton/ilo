use crate::types;
use thiserror::Error;
//use tracing::{event, instrument, Level};

#[non_exhaustive]
#[derive(Error, Debug)] //, PartialEq)]
pub enum Error {
    /*
    #[error("failed to parse int")]
    IntError(#[from] std::num::ParseIntError),
    */
    #[error("invalid utf8 when parsing to str")]
    StrUtf8Error(#[from] std::str::Utf8Error),
    #[error("invalid utf8 when parsing to string `{0}`")]
    StringUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("invalid type `{0}`")]
    TypeError(#[from] types::Error),
}

// convert attribute to type
pub trait RibclInto<T> {
    fn ribcl_into(&self) -> Result<Option<T>, Error>;
}
