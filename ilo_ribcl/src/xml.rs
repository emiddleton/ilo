use regex::Regex;
use std::{io::BufRead, str};
use thiserror::Error;
use tracing::{event, Level};

pub type Element<'a> = quick_xml::events::BytesStart<'a>;
pub type Attribute<'a> = quick_xml::events::attributes::Attribute<'a>;
pub type Event<'a> = quick_xml::events::Event<'a>;
pub type Reader<B> = quick_xml::Reader<B>;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    #[error("the {name:?} element was not found in document")]
    ElementNotFound { name: String },
    #[error("{status:?} {message:?}")]
    Response { status: u16, message: String },
    #[error("unable to read new event `{0}`")]
    ReadingEvent(#[from] quick_xml::Error),
    #[error("document root not found")]
    DocRootNotFound,
    #[error("invalid utf8 in tag name {value:x?}: {error:?}")]
    InvalidUtf8InTagName {
        value: Vec<u8>,
        error: str::Utf8Error,
    },
    #[error("invalid utf8 in attribute {value:x?}: {error:?}")]
    InvalidUtf8InAttribute {
        value: Vec<u8>,
        error: std::str::Utf8Error,
    },
    #[error("invalid response status value {value:?}: {error:?}")]
    InvalidResponseStatus {
        value: String,
        error: std::num::ParseIntError,
    },
    #[error("couldn't find child element {tag_name:?}")]
    ChildElementNotFound { tag_name: &'static str },
    #[error("invalid tag regex `{0}`")]
    RegexError(#[from] regex::Error),
}

pub struct XmlCursor<B: BufRead> {
    pub doc: String,
    pub reader: Reader<B>,
}

impl<B: BufRead> std::fmt::Debug for XmlCursor<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XmlCursor").finish()
    }
}

pub fn handle_ribcl_response_errors(element: Element<'_>) -> Result<(), Error> {
    let mut status: u16 = 0;
    let mut message: String = String::new();
    let element = element.into_owned();
    for attribute in element.attributes() {
        if let Ok(attribute) = attribute {
            let value = attribute.value.into_owned();
            let value = match std::str::from_utf8(&value) {
                Ok(v) => v,
                Err(error) => return Err(Error::InvalidUtf8InAttribute { value, error }),
            };
            let key = match str::from_utf8(&attribute.key) {
                Ok(v) => v,
                Err(error) => {
                    return Err(Error::InvalidUtf8InAttribute {
                        value: attribute.key.to_vec(),
                        error,
                    })
                }
            };

            match key.to_ascii_lowercase().as_str() {
                "status" => {
                    status = match u16::from_str_radix(value.trim_start_matches("0x"), 16) {
                        Ok(v) => v,
                        Err(error) => {
                            return Err(Error::InvalidResponseStatus {
                                value: value.to_string(),
                                error,
                            })
                        }
                    }
                }
                "message" => message = String::from(value),
                _ => {}
            }
        }
    }
    if status > 0 {
        return Err(Error::Response { status, message });
    }
    Ok(())
}

impl<'a> XmlCursor<&'a [u8]> {
    pub fn new(doc: &'a str) -> Result<(Self, Event), Error> {
        let mut reader = Reader::from_str(doc);
        let parent: Event;
        let mut buf = Vec::new();
        loop {
            let event = reader.read_event(&mut buf)?;
            match event.clone() {
                Event::Start(_) => {
                    parent = event.into_owned();
                    break;
                }
                Event::Eof => return Err(Error::DocRootNotFound),
                _ => {}
            }
            buf.clear();
        }
        Ok((
            XmlCursor {
                doc: doc.to_string(),
                reader,
            },
            parent,
        ))
    }

    pub fn find_in_children(
        &mut self,
        parent: Event,
        resp_tag: &'static str,
    ) -> Result<Event<'a>, Error> {
        let element = match parent {
            crate::xml::Event::Start(ref elem) | crate::xml::Event::Empty(ref elem) => {
                elem.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = element.name().to_ascii_lowercase();
        let mut buf = Vec::new();
        let result: Event;
        let resp_tag_regex = Regex::new(resp_tag)?;
        loop {
            let element = self.reader.read_event(&mut buf)?;
            match element.clone() {
                Event::Start(elem) | Event::Empty(elem) => {
                    let tag_bytes = elem.name();
                    let current_tag_name = match str::from_utf8(&tag_bytes) {
                        Ok(tag_name) => tag_name,
                        Err(error) => {
                            return Err(Error::InvalidUtf8InTagName {
                                value: tag_bytes.to_vec(),
                                error,
                            })
                        }
                    };
                    match current_tag_name.to_ascii_lowercase().as_str() {
                        "ribcl" => {}
                        "response" => handle_ribcl_response_errors(elem.into_owned())?,
                        ctn if resp_tag_regex.is_match(ctn) => {
                            result = element.into_owned();
                            break;
                        }
                        ignored_tag_name => {
                            event!(Level::DEBUG, ignored_tag_name);
                        }
                    }
                }
                Event::End(ref element) if element.name().to_ascii_lowercase() == break_on => {
                    return Err(Error::ChildElementNotFound { tag_name: resp_tag });
                }
                Event::Eof => return Err(Error::ChildElementNotFound { tag_name: resp_tag }),
                _ => {}
            }
            buf.clear();
        }

        Ok(result)
    }
}

#[macro_export]
macro_rules! ribcl_tag_empty {
    ($writer:ident, $src:ident, $field:ident) => {{
        if let Some($field) = &$src.$field {
            //use std::fmt::Write;
            write!($writer, "<{}/>", $field.into_ribcl()?)?;
        }
    }};
}

#[macro_export]
macro_rules! ribcl_required_tag {
    ($writer:ident, $src:ident, $field:ident) => {{
        write!($writer, "<{} value=\"{}\"/>", stringify!($field), &$src.$field.into_ribcl()?)?;
    }};
    ($writer:ident, $src:ident, $field:ident : $tag:ident) => {{
        crate::ribcl_tag!($writer, $src, $field : $tag.value);
    }};
    ($writer:ident, $src:ident, $field:ident : $tag:ident.$attr:ident) => {{
        write!($writer, "<{} {}=\"{}\"/>", stringify!($tag), stringify!($attr), &$src.$field.into_ribcl()?)?;
    }};
    ($writer:ident, $src:ident, $sf:ident { $( $field:ident ),* }) => {{
        //use std::fmt::Write;
        write!($writer, "<")?;
        write!($writer, stringify!($sf))?;
        $(
            write!($writer, " ")?;
            write!($writer, stringify!($field))?;
            write!($writer, "=\"{}\"", &$src.$sf.$field.into_ribcl()?)?;
        )*
        write!($writer, "/>")?;
    }};
    ($writer:ident, $src:ident, $sf:ident : $tag:ident { $( $field:ident ),* }) => {{
        write!($writer, "<")?;
        write!($writer, stringify!($tag))?;
        $(
            write!($writer, " ")?;
            write!($writer, stringify!($field))?;
            write!($writer, "=\"{}\"", &$sf.$field.into_ribcl()?)?;
        )*
        write!($writer, "/>")?;
    }};
    ($writer:ident, $src:ident, $sf:ident : $tag:ident { $( $field:ident : $attr_key:ident ),* }) => {{
        if let Some($sf) = &$src.$sf {
            use std::fmt::Write;
            write!($writer, "<")?;
            write!($writer, stringify!($tag))?;
            $(
                write!($writer, " ")?;
                write!($writer, stringify!($attr_key))?;
                write!($writer, "=\"{}\"", &$sf.$field.into_ribcl()?)?;
            )*
            write!($writer, "/>")?;
        }
    }};
}

#[macro_export]
macro_rules! ribcl_tag {
    ($writer:ident, $src:ident, $field:ident) => {{
        if let Some($field) = &$src.$field {
            write!($writer, "<{} value=\"{}\"/>", stringify!($field), $field.into_ribcl()?)?;
        }
    }};
    ($writer:ident, $src:ident, $field:ident) => {{
        if let Some($field) = &$src.$field {
            write!($writer, "<{} value=\"{}\"/>", stringify!($field), $field.into_ribcl()?)?;
        }
    }};
    ($writer:ident, $src:ident, $field:ident : $tag:ident) => {{
        crate::ribcl_tag!($writer, $src, $field : $tag.value);
    }};
    ($writer:ident, $src:ident, $field:ident : $tag:ident.$attr:ident) => {{
        if let Some($field) = &$src.$field {
            write!($writer, "<{} {}=\"{}\"/>", stringify!($tag), stringify!($attr), $field.into_ribcl()?)?;
        }
    }};
    ($writer:ident, $src:ident, $sf:ident { $( $field:ident ),* }) => {{
        if let Some($sf) = &$src.$sf {
            write!($writer, "<")?;
            write!($writer, stringify!($sf))?;
            $(
                write!($writer, " ")?;
                write!($writer, stringify!($field))?;
                write!($writer, "=\"{}\"", $sf.$field.into_ribcl()?)?;
            )*
            write!($writer, "/>")?;
        }
    }};
    ($writer:ident, $src:ident, $sf:ident : $tag:ident { $( $field:ident ),* }) => {{
        if let Some($sf) = &$src.$sf {
            write!($writer, "<")?;
            write!($writer, stringify!($tag))?;
            $(
                write!($writer, " ")?;
                write!($writer, stringify!($field))?;
                write!($writer, "=\"{}\"", $sf.$field.into_ribcl()?)?;
            )*
            write!($writer, "/>")?;
        }
    }};
    ($writer:ident, $src:ident, $sf:ident : $tag:ident { $( $field:ident : $attr_key:ident ),* }) => {{
        if let Some($sf) = &$src.$sf {
            write!($writer, "<")?;
            write!($writer, stringify!($tag))?;
            $(
                write!($writer, " ")?;
                write!($writer, stringify!($attr_key))?;
                write!($writer, "=\"{}\"", &$sf.$field.into_ribcl()?)?;
            )*
            write!($writer, "/>")?;
        }
    }};
}

#[macro_export]
macro_rules! ribcl_header {
    ($writer:ident, $credentials:expr, $section_name:ident, $mode:ident) => {{
        write!(
            $writer,
            "<?xml version=\"1.0\"?>\
                <ribcl version=\"2.0\">\
                    <login user_login=\"{}\" password=\"{}\">\
                        <{} mode=\"{}\">",
            &$credentials.username,
            &$credentials.password,
            &stringify!($section_name),
            &stringify!($mode)
        )
    }};
    ($writer:ident, $credentials:expr, $section_name:ident, $mode:ident, $command_name:ident) => {{
        write!(
            $writer,
            "<?xml version=\"1.0\"?>\
                <ribcl version=\"2.0\">\
                    <login user_login=\"{}\" password=\"{}\">\
                        <{} mode=\"{}\">\
                            <{}>",
            &$credentials.username,
            &$credentials.password,
            &stringify!($section_name),
            &stringify!($mode),
            &stringify!($command_name)
        )
    }};
}

#[macro_export]
macro_rules! ribcl_footer {
    ($writer:ident, $section_name:ident) => {{
        write!(
            $writer,
            "</{}></login></ribcl>\r\n",
            &stringify!($section_name)
        )
    }};
    ($writer:ident, $section_name:ident, $command_name:ident) => {{
        write!(
            $writer,
            "</{}></{}></login></ribcl>\r\n",
            &stringify!($command_name),
            &stringify!($section_name)
        )
    }};
}

#[cfg(test)]
mod tests {}
