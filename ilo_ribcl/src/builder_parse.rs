use crate::{
    builder_parse, ribcl_into, types, xml,
    xml::{Event, XmlCursor},
};
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    io::BufRead,
    str,
};
use thiserror::Error;
use tracing::{event, instrument, Level};

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("\"{target}\" was not found in this response")]
    NotFound { target: &'static str },
    #[error("invalid utf8 when parsing to str `{0}`")]
    StrUtf8Error(#[from] str::Utf8Error),
    #[error("invalid utf8 when parsing to String `{0}`")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("type error occurred `{0}`")]
    TypeError(#[from] types::Error),
    #[error("invalid xml `{0:?}`")]
    InvalidXml(#[from] quick_xml::Error),
    #[error("error in xml `{0}`")]
    XmlError(#[from] xml::Error),
    #[error("ribcl into `{0}`")]
    RibclInto(#[from] ribcl_into::Error),
    #[error("expected one attribute found more then one")]
    MoreThenOneAttributes,
    #[error("a fmt error occurred `{0}`")]
    FmtError(#[from] fmt::Error),
}

pub trait BuilderParse<'a, T> {
    fn builder_parse(&mut self, parent: Event<'a>, builder: Option<T>) -> Result<T, Error>;
}

// handle optional elements don't allow updating
impl<'a, B, T> BuilderParse<'_, Option<T>> for XmlCursor<B>
where
    T: std::fmt::Debug,
    XmlCursor<B>: BuilderParse<'a, T> + std::fmt::Debug,
    B: BufRead,
{
    #[instrument]
    fn builder_parse(
        &mut self,
        parent: Event<'_>,
        _builder: Option<Option<T>>,
    ) -> Result<Option<T>, Error> {
        match self.builder_parse(parent.into_owned(), None) {
            Ok(builder) => Ok(Some(builder)),
            Err(Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

#[derive(Default, Debug)]
pub struct VecBuilder<T: Default>(pub Vec<T>);

// handle parsing a vector of attributes
//
impl<'a, B, C> BuilderParse<'_, VecBuilder<C>> for XmlCursor<B>
where
    B: BufRead,
    C: Default + std::fmt::Debug,
    XmlCursor<B>: BuilderParse<'a, C>,
{
    fn builder_parse(
        &mut self,
        parent: Event<'_>,
        builder: Option<VecBuilder<C>>,
    ) -> Result<VecBuilder<C>, Error> {
        let mut builder = builder.unwrap_or_else(VecBuilder::default);
        let element = match parent {
            Event::Start(ref elem) | Event::Empty(ref elem) => elem.clone().into_owned(),
            _ => unreachable!(),
        };
        let break_on = element.name().to_ascii_lowercase();
        let mut buf = Vec::new();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event!(Level::DEBUG, break_on = ?break_on_value);
        event!(Level::DEBUG, "ENTERING Vec -> {:?}", &builder);
        loop {
            let event = self.reader.read_event(&mut buf)?;
            match event.clone() {
                Event::Start(elem) | Event::Empty(elem) => {
                    event!(Level::DEBUG, element=?elem);
                    let contained: C = self.builder_parse(event.into_owned(), None)?;
                    builder.0.push(contained);
                }
                // return on closing tab
                Event::End(ref elem) if elem.name().to_ascii_lowercase() == break_on => {
                    event!(Level::DEBUG, "BREAKING Vec");
                    break;
                }
                Event::Eof => {
                    event!(Level::DEBUG, "BREAKING Vec Eof");
                    return Err(builder_parse::Error::NotFound { target: "Vec" });
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(builder)
    }
}

impl<B, C> TryFrom<VecBuilder<B>> for Vec<C>
where
    C: TryFrom<B>,
    B: Default + std::fmt::Debug,
    Error: From<<C as TryFrom<B>>::Error>,
{
    type Error = Error;
    fn try_from(builder: VecBuilder<B>) -> Result<Vec<C>, Self::Error> {
        let mut output = vec![];
        event!(Level::DEBUG, builder=?builder, "Vec try_from<VecBuilder<B>>");
        for b in builder.0 {
            event!(Level::DEBUG, builder=?b);
            output.push(b.try_into()?)
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {}
