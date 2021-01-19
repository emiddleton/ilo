use crate::{builder_parse, client, into_ribcl, write_ribcl};
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use thiserror::Error;

/// Errors occurring while making API calls.
#[non_exhaustive]
#[derive(Error, Debug)] //, PartialEq)]
pub enum Error {
    /// The Endpoint does not support this API call
    #[error("requires {requirements}")]
    NotSupported {
        /// description of the firmware version requirements
        requirements: &'static str,
    },

    /// An error occurred creating builder
    #[error("builder parse error: {source}")]
    BuilderParse {
        /// name of the builder
        target: &'static str,
        /// error that occurred while creating builder
        #[source]
        source: builder_parse::Error,
        #[cfg(feature = "backtrace")]
        #[backtrace]
        backtrace: Backtrace,
    },

    /// An error occurred converting struct for API command
    #[error("write ribcl error: {source}")]
    WriteRibcl {
        #[from]
        source: write_ribcl::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /// Error occurred converting value for use in API call
    #[error("into ribcl error: {source}")]
    IntoRibcl {
        #[from]
        source: into_ribcl::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /// An error occurred formatting API command
    #[error("string format error: `{:#?}`", source)]
    StrFormat {
        #[from]
        source: std::fmt::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /// Error loading file for command
    #[error("io error: `{:#?}`", source)]
    Io {
        #[from]
        source: std::io::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },

    /// Error occurred sending command to Endpoint
    #[error("client error: {source}")]
    Client {
        #[from]
        source: client::Error,
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace,
    },
}

macro_rules! version_geq {
    ($left:ident, $right:literal) => {{
        let left_itr = $left
            .clone()
            .unwrap()
            .split(".")
            .map(|l| l.parse().unwrap())
            .collect::<Vec<u64>>()
            .into_iter();
        let right_itr = $right
            .split(".")
            .map(|l| l.parse().unwrap())
            .collect::<Vec<u64>>()
            .into_iter();
        left_itr
            .zip(right_itr)
            .find_map(|(l, r)| {
                if l > r {
                    Some(true)
                } else if l < r {
                    Some(false)
                } else {
                    None
                }
            })
            .unwrap_or(true)
    }};
}

macro_rules! fw_is {
    ($version:tt) => {
        |firmware| match firmware {
            &crate::types::FwVersion {
                management_processor: Some(crate::types::Version::$version),
                ..
            } => true,
            _ => false,
        }
    };
    ($version:tt, $geq_version:literal) => {
        |firmware| match firmware {
            &crate::types::FwVersion {
                management_processor: Some(crate::types::Version::$version),
                firmware_version: ref sv,
                ..
            } if version_geq!(sv, $geq_version) => true,
            _ => false,
        }
    };
}

macro_rules! assert_fw {
    ( $firmware:expr, ) => {};
    ( $firmware:expr, $requirements_msg:literal, $( ( $( $condition_closure:tt ),+ ) ),* ) => {{
        if let Some(fw) = $firmware.as_ref() {
                let results = vec![ $( fw_is!($( $condition_closure ),*)(fw) ),* ];
                if results.len() > 0 && !results.iter().any(|&result|result) {
                  Err($crate::commands::Error::NotSupported { requirements: $requirements_msg })
                } else {
                    Ok(())
                }
            } else {
                Ok(())
        }?
    }};
}

macro_rules! ribcl_command {
    (@inner $writer:ident, $credentials:expr, $section_name:ident, $mode:ident, $command_name:literal) => {{
        ribcl_header!($writer, $credentials, $section_name, $mode)?;
        write!($writer, "<{}/>", $command_name)?;
        ribcl_footer!($writer, $section_name)?;
    }};
    (@inner $writer:ident, $credentials:expr, $section_name:ident, $mode:ident, $command_name:ident) => {{
        ribcl_header!($writer, $credentials, $section_name, $mode)?;
        write!($writer, "<{}/>", stringify!($command_name))?;
        ribcl_footer!($writer, $section_name)?;
    }};
    (@inner $writer:ident, $credentials:expr, $section_name:ident, $mode:ident, $command_name:ident, $attr_name:literal, $value:ident) => {{
        ribcl_header!($writer, $credentials, $section_name, $mode)?;
        {
            use $crate::into_ribcl::IntoRibcl;
            write!(
                $writer,
                "<{} {}=\"{}\"/>",
                stringify!($command_name),
                $attr_name,
                $value.into_ribcl()?
            )?
        };
        ribcl_footer!($writer, $section_name, $command_name)?;
    }};
    (@inner $writer:ident, $credentials:expr, $section_name:ident, $mode:ident, $command_name:ident, $command:expr) => {{
        ribcl_header!($writer, $credentials, $section_name, $mode, $command_name)?;
        $command;
        ribcl_footer!($writer, $section_name, $command_name)?;
    }};
    ($($all:tt)+) => {
        use std::fmt::Write;
        use $crate::{ribcl_header,ribcl_footer};
        ribcl_command!(@inner $($all)+)
    };
}

macro_rules! ribcl_parse_response {
    ($response:ident, $resp_tag_regex:literal -> $($ret_type:tt)+) => {
        ribcl_parse_response!(@final $response [ $resp_tag_regex ] [ $($ret_type)+ ])
    };
    ($response:ident, $($resp_tag_regex:tt)+ -> $($ret_type:tt)+) => {
        ribcl_parse_response!(@final $response [ stringify!($($resp_tag_regex)+) ] [ $($ret_type)+ ])
    };
    ($response:ident) => {
        ribcl_parse_response!(@final $response [""] [()])
    };
    (@item_type_name $ret_type:ty) => {
        stringify!(ret_type)
    };
    (@item_type_name types::$ret_type:tt) => {
        stringify!(types::$ret_type)
    };
    (@item_type_name Vec<types::$ret_type:tt>) => {
        stringify!(types::$ret_type)
    };

    (@type_name $($ret_type:tt)*) => {
        stringify!($($ret_type)*)
    };

    // get builder for return type
    (@ret_type_builder [$($ret_type_path:tt)*] $ret_type:tt >) => {
        paste::paste!{ $($ret_type_path)*[<$ret_type:camel Builder>] > }
    };
    (@ret_type_builder [$($ret_type_path:tt)*] $ret_type:tt) => {
        paste::paste!{ $($ret_type_path)*[<$ret_type:camel Builder>] }
    };
    (@ret_type_builder [] Vec < $($tail:tt)+) => {
        ribcl_parse_response!(@ret_type_builder [$crate::builder_parse::VecBuilder <] $($tail)+)
    };
    (@ret_type_builder [$($ret_type_path:tt)*] $ret_token:tt $($tail:tt)+) => {
        ribcl_parse_response!(@ret_type_builder [$($ret_type_path)* $ret_token] $($tail)+)
    };
    (@type_builder ()) => {
        $crate::types::NullBuilder
    };
    (@type_builder $($ret_type:tt)+) => {
        ribcl_parse_response!(@ret_type_builder [] $($ret_type)+)
    };

    (@tag_type [ $response_tag_regex:literal ]) => {
        $response_tag_regex
    };
    (@tag_type [ $($response_tag:tt)+ ] ) => {
        stringify!($($response_tag)+)
    };

    (@final $response:ident [ $($resp_tag_regex:tt)+ ] [ $($ret_type:tt)* ]) => {{
        use $crate::{builder_parse, builder_parse::BuilderParse, commands, xml};
        use std::convert::TryInto;

        let mut doc_iter = $response.split(r#"<?xml version="1.0"?>"#);
        doc_iter
            .find_map(|doc| {
                tracing::event!(tracing::Level::TRACE, doc);
                if doc.is_empty() {
                    return None;
                }

                tracing::event!(tracing::Level::DEBUG, document=doc);
                let (mut xml_cursor, root) = match $crate::xml::XmlCursor::new(doc) {
                    Ok(val) => val,
                    Err(err) => return Some(Err(err.into())),
                };
                let response_tag = ribcl_parse_response!(@tag_type [$($resp_tag_regex)+]);

                tracing::event!(tracing::Level::DEBUG, searching_for=response_tag);
                let elem = match xml_cursor.find_in_children(root, response_tag) {
                    Ok(val) => val,
                    Err(xml::Error::ChildElementNotFound{..}) => return None,
                    Err(err) => return Some(Err(err.into())),
                };

                tracing::event!(tracing::Level::DEBUG, found_root_element=?elem);
                let result: Result<$($ret_type)*, builder_parse::Error> = (|| {
                        let builder : ribcl_parse_response!(@type_builder $($ret_type)*) = xml_cursor.builder_parse(elem, None)?;
                        builder.try_into()
                })();

                tracing::event!(tracing::Level::DEBUG, found_result=?result);
                match result {
                    Ok(result) => Some(Ok(result)),
                    Err(builder_parse::Error::NotFound { target }) if ribcl_parse_response!(@item_type_name $($ret_type)*) == target => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .ok_or(builder_parse::Error::NotFound {
                target: ribcl_parse_response!(@type_name $($ret_type)*),
            })
            .map_err(|source|
                commands::Error::BuilderParse {
                    target: ribcl_parse_response!(@type_name $($ret_type)*),
                    source,
            })
    }};
 }

macro_rules! get_method{
    (
        @final
        [$(#[$outer:meta])*]
        $mod:ident.$fn_name:tt -> [$($resp_tag:tt)+] : [$($ret_type:tt)+] [$(, $requirements_msg:literal, $( ( $($conditions:tt),+ ) ),+)?]
    ) => {
        $(#[$outer])*
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(&mut self) -> Result<$($ret_type)+, crate::commands::Error> {
            assert_fw!(self.firmware(), $($requirements_msg, $( (  $($conditions),+ ) ),*)*);
            let mut request = String::new();
            ribcl_command!(
                request,
                self.auth(),
                $mod,
                read,
                $fn_name
            );
            let response = self.send(request.into_bytes()).await?;
            Ok(
                ribcl_parse_response!(@final response [$($resp_tag)+] [$($ret_type)+])?.map_err(
                    |source| $crate::commands::Error::BuilderParse{
                        target:stringify!($($ret_type)+),
                        source
                    }
                )?
            )
        }

    };
    (
        @parse_ret_type
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] : [$($resp_type_tokens:tt)+] {}
    ) => {
        get_method!(
            @final
            [$(#[$outer])*]
            $mod.$fn_name -> [$($resp_tag_tokens)+] : [ $($resp_type_tokens)+ ] []
        );
    };
    (
        @parse_ret_type
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] : [$($resp_type_tokens:tt)+] {, $requirements:literal $($tail:tt)+ }
    ) => {
        get_method!(
            @final
            [$(#[$outer])*]
            $mod.$fn_name -> [$($resp_tag_tokens)+] : [ $($resp_type_tokens)+ ] [, $requirements $($tail)+]
        );
    };

    // parsing response type
    (
        @parse_ret_type
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] : [$($type_tokens:tt)+] { $type_token:tt $($tail:tt)*}
    ) => {
        get_method!(
            @parse_ret_type
            [$(#[$outer])*]
            $mod.$fn_name -> [ $($resp_tag_tokens)+ ] : [ $($type_tokens)+ $type_token ] { $($tail)* }
        );
    };
    // start parsing response type
    (
        @parse_ret_type
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] : [] { $type_token:tt $($tail:tt)* }
    ) => {
        get_method!(
            @parse_ret_type
            [$(#[$outer])*]
            $mod.$fn_name -> [$($resp_tag_tokens)+] : [$type_token] {$($tail)*}
        );
    };

    // : ends response tag tokens load
    (
        @parse_ret_tag
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] { : $($tail:tt)+}
    ) => {
        get_method!(
            @parse_ret_type
            [$(#[$outer])*]
            $mod.$fn_name -> [$($resp_tag_tokens)+] : [] {$($tail)*}
        );
    };

    // missing return tag use function name and not requirements assumes not
    (
        @parse_ret_tag
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] {}
    ) => {
        get_method!(
            @final
            [$(#[$outer])*]
            $mod.$fn_name -> [$fn_name] : [$($resp_tag_tokens)+] []
        );
    };
    // missing return tag use function name
    (
        @parse_ret_tag
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)+] {, $requirements:literal, $($tail:tt)+}
    ) => {
        get_method!(
            @final
            [$(#[$outer])*]
            $mod.$fn_name -> [$fn_name] : [$($resp_tag_tokens)+] [, $requirements, $($tail)+]
        );
    };
    // loading return tag tokens
    (
        @parse_ret_tag
        [$(#[$outer:meta])*]
        $mod:tt.$fn_name:tt -> [$($resp_tag_tokens:tt)*] { $resp_tag_token:tt $($tail:tt)*}
    ) => {
        get_method!(
            @parse_ret_tag
            [$(#[$outer])*] $mod.$fn_name -> [$($resp_tag_tokens)*$resp_tag_token]
            {$($tail)*}
        );
    };
    (
        $(#[$outer:meta])+
        $mod:tt.$fn_name:tt -> $($tail:tt)+
    ) => {
        get_method!(@parse_ret_tag [$(#[$outer])*] $mod.$fn_name -> [] {$($tail)+});
    };

}
macro_rules! mod_method {
    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident : $tag_name:literal
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(
            &mut self,
        ) -> Result<(), crate::commands::Error> {
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $tag_name);
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok(()),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };

    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident ( $arg_type:ty )
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(
            &mut self,
            arg: $arg_type,
        ) -> Result<$arg_type, crate::commands::Error> {
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $fn_name, {
                use crate::write_ribcl::WriteRibcl;
                arg.write_ribcl(&mut request)?;
            });
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok((arg)),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };

    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(&mut self) -> Result<(), crate::commands::Error> {
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $fn_name);
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok(()),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };

    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident ( $arg_type:ty ), $requirements_msg:literal, $( ( $($conditions:tt),* ) ),*
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(&mut self, arg: $arg_type) -> Result<$arg_type, crate::commands::Error> {
            assert_fw!(self.firmware(), $requirements_msg, $( (  $($conditions),* ) ),*);
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $fn_name, {
                use $crate::write_ribcl::WriteRibcl;
                arg.write_ribcl(&mut request)?;
            });
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok((arg)),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };

    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident ($attr_name:literal : $($arg_type:ty)+ ) $(, $requirements_msg:literal, $( ( $($conditions:tt),* ) ),*)?
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(&mut self, arg: $($arg_type)+) -> Result<$($arg_type)+, crate::commands::Error> {
            assert_fw!(self.firmware(), $($requirements_msg, $( (  $($conditions),+ ) ),*)*);
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $fn_name, $attr_name, arg);
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok((arg)),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };
    (
        $(#[$outer:meta])+
        $mod:ident.$fn_name:ident, $requirements_msg:literal, $( ( $($conditions:tt),* ) ),*
    ) => {
        $(#[$outer])+
        #[tracing::instrument(skip(self))]
        pub async fn $fn_name(&mut self) -> Result<(), crate::commands::Error> {
            assert_fw!(self.firmware(), $requirements_msg, $( (  $($conditions),* ) ),*);
            let mut request = String::new();
            ribcl_command!(request, self.auth(), $mod, write, $fn_name);
            let response = self.send(request.into_bytes()).await?;
            match ribcl_parse_response!(response) {
                Ok(_) | Err(crate::commands::Error::BuilderParse{
                    source: crate::builder_parse::Error::NotFound{target: _}, ..
                }) => Ok(()),
                Err(err) => {
                    Err(err)
                },
            }
        }
    };
}
