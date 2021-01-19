use crate::{write_ribcl, write_ribcl::WriteRibcl};
use std::str;
use thiserror::Error;
use tracing::instrument;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid utf8 when parsing to str `{0}`")]
    StrUtf8Error(#[from] str::Utf8Error),
    #[error("write ribcl error `{0}`")]
    WriteRibclError(#[from] Box<write_ribcl::Error>),
}

pub trait IntoRibcl {
    fn into_ribcl(&self) -> Result<String, Error>;
}

// handle vectors by running through each element
impl<T: WriteRibcl + std::fmt::Debug> IntoRibcl for Vec<T> {
    #[instrument]
    fn into_ribcl(&self) -> Result<String, Error> {
        let mut result = String::new();
        for t in self.iter() {
            match t.write_ribcl(&mut result) {
                Ok(ok) => ok,
                Err(e) => return Err(Error::WriteRibclError(Box::new(e))),
            };
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {}
