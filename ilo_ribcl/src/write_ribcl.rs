use crate::into_ribcl;
use std::fmt;
use thiserror::Error;
//use tracing::{event, instrument, Level};

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("a fmt error occurred `{0}`")]
    FmtError(#[from] fmt::Error),
    #[error("into ribcl error `{0}`")]
    IntoRibcl(#[from] into_ribcl::Error),
}

pub trait WriteRibcl {
    fn write_ribcl<W: std::fmt::Write>(&self, writer: &mut W) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {}
