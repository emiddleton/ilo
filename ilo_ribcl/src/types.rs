use crate::ribcl_into::RibclInto;
use chrono::naive::{NaiveDate, NaiveDateTime};
use ilo_ribcl_derive::BuilderParse;
use inflector::Inflector;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, fmt, net::Ipv4Addr, str};
use thiserror::Error;
use tracing::{event, Level};

/// Errors returned parsing type values
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    /// The value was not supported in this field.
    #[error("invalid {target:?} string {value:?}")]
    InvalidString {
        /// the target field
        target: &'static str,
        /// the value being set
        value: String,
    },
    /// The value doesn't contain a valid ip address
    #[error("invalid ip address `{value}` `{error:?}`")]
    IpAddrError {
        /// The value being parsed
        value: String,
        /// Error raised parsing Addr
        error: std::net::AddrParseError,
    },
    /// The value does not contain a valid integer value
    #[error("failed to parse int `{value}` `{error:?}`")]
    ParseIntError {
        /// The value being parsed
        value: String,
        /// Error raised parsing integer
        error: std::num::ParseIntError,
    },
    /// The value does not contain a valid floating point value
    #[error("failed to parse float `{value}` `{error:?}`")]
    ParseFloatError {
        /// The value being parsed
        value: String,
        /// Error raised parsing float
        error: std::num::ParseFloatError,
    },
    /// The value is not a valid date time value
    #[error("could not parse date or time `{0}`")]
    DateTimeError(#[from] chrono::format::ParseError),
    /// The value contained invalid utf8 values.
    #[error("invalid utf8 when parsing to str `{0}`")]
    StrUtf8Error(#[from] std::str::Utf8Error),
    /// The value contained invalid utf8 values.
    #[error("invalid utf8 when parsing to string `{0}`")]
    StringUtf8Error(#[from] std::string::FromUtf8Error),
}

/// Wrapper for building simple single value types
#[derive(Debug)]
pub struct SimpleBuilder<T: std::fmt::Debug>(pub T);

/// Define SimpleBuilder for a basic type
#[macro_export]
macro_rules! simple_builder_type_def {
    ($ty:ident) => {
        paste::paste! {
            /// A builder type for $ty
            pub type [<$ty:camel Builder>] = crate::types::SimpleBuilder<$ty>;
        }
    };
}

/// Define try_from to convert SimpleBuilder to basic type
#[macro_export]
macro_rules! simple_builder_try_from_def {
    ($ty:ident) => {
        impl std::convert::TryFrom<crate::types::SimpleBuilder<$ty>> for $ty {
            type Error = crate::builder_parse::Error;
            fn try_from(builder: crate::types::SimpleBuilder<$ty>) -> Result<$ty, Self::Error> {
                Ok(builder.0)
            }
        }
    };
}

///
#[macro_export]
macro_rules! simple_builder_ribcl_into_def {
    ($ty:ident, {$($func:tt)*}) => {
        paste::paste! {
            impl<'a> crate::ribcl_into::RibclInto<[<$ty:camel Builder>]> for crate::xml::Attribute<'a> {
                fn ribcl_into(&self) -> Result<Option<[<$ty:camel Builder>]>, crate::ribcl_into::Error> {
                    let str_val = String::from_utf8(self.value.clone().into_owned()).map(|v|v.trim().to_string());
                    // ignore empty fields
                    if let Ok(val) = &str_val {
                        if val.is_empty() {
                            return Ok(None)
                        }
                    }
                    let val : Option<$ty> = str_val
                        .map::<Result<Option<$ty>, crate::types::Error>,_>($($func)*)??;
                    Ok(val.map($crate::types::SimpleBuilder))
                }
            }
            impl<'a> crate::ribcl_into::RibclInto<[<$ty:camel Builder>]> for String {
                fn ribcl_into(&self) -> Result<Option<[<$ty:camel Builder>]>, crate::ribcl_into::Error> {
                    // ignore empty fields
                    if self.is_empty() {
                        return Ok(None)
                    }
                    let val : Result<String, crate::types::Error> = Ok(self.clone());
                    let val : Option<$ty> = val
                        .map::<Result<Option<$ty>, crate::types::Error>,_>($($func)*)??;
                    Ok(val.map($crate::types::SimpleBuilder))
                }
            }
        }
    }
}

#[macro_export]
macro_rules! simple_builder_parse_impl {
    ($ty:ident) => {
        paste::paste! {
            // use first attribute and use to create builder
            impl<'a, B> crate::builder_parse::BuilderParse<'_, [<$ty:camel Builder>]> for crate::xml::XmlCursor<B>
            where
                B: std::io::BufRead + std::fmt::Debug,
            {
                fn builder_parse(
                    &mut self,
                    parent: crate::xml::Event<'_>,
                    _builder: Option<[<$ty:camel Builder>]>,
                ) -> Result<[<$ty:camel Builder>], crate::builder_parse::Error> {
                    use $crate::ribcl_into::RibclInto;
                    tracing::event!(tracing::Level::DEBUG, ?parent);

                    let element : crate::xml::Element = match parent {
                        crate::xml::Event::Start(ref elem) | crate::xml::Event::Empty(ref elem) => {
                            elem.clone().into_owned()
                        },
                        _ => unreachable!(),
                    };

                    let attributes: Vec<crate::xml::Attribute> =
                        element.attributes().filter_map(|a| a.ok()).collect();
                    match attributes.len() {
                        1 => {
                            Ok(attributes
                                .first()
                                .unwrap()
                                .ribcl_into()?
                                .ok_or($crate::builder_parse::Error::NotFound {
                                    target: stringify!($ty),
                                })?)
                        }
                        0 => {
                            match self.reader.read_text(element.name(), &mut Vec::new()) {
                                Ok(content) if content.trim().is_empty() => {
                                    Err($crate::builder_parse::Error::NotFound {
                                        target: stringify!($ty),
                                    }.into())
                                },
                                Ok(content) => {
                                    Ok(content
                                        .ribcl_into()?
                                        .ok_or(crate::builder_parse::Error::NotFound {
                                            target: stringify!($ty),
                                        })?)
                                },
                                Err(e) => Err(e.into()),
                            }
                        }
                        _ =>  {
                            Err(crate::builder_parse::Error::MoreThenOneAttributes)
                        }
                    }


                }
            }
        }
    };
}

#[macro_export]
macro_rules! simple_builder_into_ribcl_def {
    ($type:ty) => {
        simple_builder_into_ribcl_def!($type,{|v| format!("{}", v) });
    };
    ($type:ty, {$($func:tt)+}) => {
        impl crate::into_ribcl::IntoRibcl for $type {
            #[tracing::instrument]
            fn into_ribcl(&self) -> Result<String, crate::into_ribcl::Error> {
                Ok(self).map($($func)+)
            }
        }
    };
}

#[macro_export]
macro_rules! simple_builder_def {
    ($ty:ident, {$($func:tt)*}) => {
        paste::paste! {
            //$crate::simple_builder_type_def!($ty);
            $crate::simple_builder_ribcl_into_def!($ty, {$($func)*});
            $crate::simple_builder_into_ribcl_def!($ty);
            $crate::simple_builder_parse_impl!($ty);
            $crate::simple_builder_try_from_def!($ty);
        }
    };
    ($ty:ident, {$($fn_from:tt)*}, {$($fn_into:tt)*}) => {
        paste::paste! {
            //$crate::simple_builder_type_def!($ty);
            $crate::simple_builder_ribcl_into_def!($ty, {$($fn_from)*});
            $crate::simple_builder_into_ribcl_def!($ty, {$($fn_into)*});
            $crate::simple_builder_parse_impl!($ty);
            $crate::simple_builder_try_from_def!($ty);
        }
    };
}

#[macro_export]
macro_rules! simple_builder_alias {
    ($ty:ident, $ty_base:ident) => {
        pub type $ty = $ty_base;
        $crate::simple_builder_type_def!($ty);
    };
}

// number simple types
pub type U32Builder = SimpleBuilder<u32>;
simple_builder_def!(u32, {
    |value| match value.parse::<u32>() {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let v_lowercase = value.to_lowercase();
            if ["n/a"].iter().any(|v| v == &v_lowercase) {
                Ok(None)
            } else {
                Err(crate::types::Error::ParseIntError { value, error })
            }
        }
    }
});
//simple_builder_alias!(Minutes, u32);
pub type Minutes = u32;
pub type MinutesBuilder = SimpleBuilder<Minutes>;

pub type U16Builder = SimpleBuilder<u16>;
simple_builder_def!(u16, {
    |value| match value.parse::<u16>() {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let v_lowercase = value.to_lowercase();
            if ["n/a"].iter().any(|v| v == &v_lowercase) {
                Ok(None)
            } else {
                Err(crate::types::Error::ParseIntError { value, error })
            }
        }
    }
});
//simple_builder_alias!(Port, u16);
pub type Port = u16;
pub type PortBuilder = SimpleBuilder<Port>;

pub type F64Builder = SimpleBuilder<f64>;
simple_builder_def!(f64, {
    |value| match value.parse::<f64>() {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let v_lowercase = value.to_lowercase();
            if ["n/a"].iter().any(|v| v == &v_lowercase) {
                Ok(None)
            } else {
                Err(crate::types::Error::ParseFloatError { value, error })
            }
        }
    }
});

// String simple types
pub type StringBuilder = SimpleBuilder<String>;
simple_builder_def!(String, {
    |value| {
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.trim().to_string()))
        }
    }
});

//simple_builder_alias!(MacAddress, String);
pub type MacAddress = String;
pub type MacAddressBuilder = SimpleBuilder<MacAddress>;

//simple_builder_alias!(HostName, String);
pub type HostName = String;
pub type HostNameBuilder = SimpleBuilder<HostName>;

//simple_builder_alias!(DomainName, String);
pub type DomainName = String;
pub type DomainNameBuilder = SimpleBuilder<DomainName>;

//simple_builder_alias!(FQDN, String);
pub type FQDN = String;
pub type FQDNBuilder = SimpleBuilder<FQDN>;

//simple_builder_alias!(Timezone, String);
pub type Timezone = String;
pub type TimezoneBuilder = SimpleBuilder<Timezone>;

//simple_builder_alias!(Url, String);
pub type Url = String;
pub type UrlBuilder = SimpleBuilder<Url>;

//simple_builder_alias!(CertificateSigningRequest, String);
pub type CertificateSigningRequest = String;
pub type CertificateSigningRequestBuilder = SimpleBuilder<CertificateSigningRequest>;

//simple_builder_alias!(Certificate, String);
pub type Certificate = String;
pub type CertificateBuilder = SimpleBuilder<Certificate>;

//simple_builder_alias!(Null, String);
type Null = ();
pub type NullBuilder = SimpleBuilder<Null>;
simple_builder_def!(Null, { |_| Ok(Some(())) }, { |_| String::new() });

pub type NaiveDateTimeBuilder = SimpleBuilder<NaiveDateTime>;
simple_builder_def!(NaiveDateTime, {
    |value| match value.to_ascii_lowercase().as_str() {
        "[not set]" => Ok(None),
        _ => {
            match ["%m/%d/%Y %H:%M", "%a %b %e %H:%M:%S %Y"]
                .iter()
                .find_map(|fmt| NaiveDateTime::parse_from_str(value.as_str(), fmt).ok())
            {
                None => match NaiveDate::parse_from_str(value.as_str(), "%m-%d-%Y") {
                    Err(_) => Err(Error::InvalidString {
                        target: "NaiveDateTime",
                        value,
                    }),
                    Ok(val) => Ok(Some(val.and_hms(0, 0, 0))),
                },
                val => Ok(val),
            }
        }
    }
});

pub type NaiveDateBuilder = SimpleBuilder<NaiveDate>;
simple_builder_def!(NaiveDate, {
    |value| {
        if "[not set]" == value.to_ascii_lowercase().as_str() {
            return Ok(None);
        }
        ["%m/%d/%Y", "%b %d %Y", "%m-%d-%Y"]
            .iter()
            .find_map(|fmt| NaiveDate::parse_from_str(value.as_str(), fmt).ok())
            .ok_or(Error::InvalidString {
                target: "NaiveDate",
                value,
            })
            .map(Some)
    }
});

// handle boolean
pub type BoolBuilder = SimpleBuilder<bool>;
simple_builder_def!(
    bool,
    {
        |value| match value.to_ascii_lowercase().as_str() {
            "disabled" | "n" | "no" | "f" => Ok(Some(false)),
            "enabled" | "y" | "yes" | "t" => Ok(Some(true)),
            _ => Err(Error::InvalidString {
                target: "bool",
                value,
            }),
        }
    },
    {
        |&value| match value {
            true => String::from("Y"),
            false => String::from("N"),
        }
    }
);

impl std::default::Default for BoolBuilder {
    fn default() -> Self {
        SimpleBuilder(false)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Ip4Address(Ipv4Addr);
pub type Ip4AddressBuilder = SimpleBuilder<Ip4Address>;
simple_builder_def!(Ip4Address, {
    |value| match value.parse() {
        Ok(address) => {
            if address == Ipv4Addr::new(0, 0, 0, 0) {
                Ok(None)
            } else {
                Ok(Some(Ip4Address(address)))
            }
        }
        Err(error) => {
            let v_lowercase = value.to_lowercase();
            if ["unknown", "n/a"].iter().any(|v| v == &v_lowercase) {
                Ok(None)
            } else {
                Err(crate::types::Error::IpAddrError { value, error })
            }
        }
    }
});
//simple_builder_alias!(Ip4SubnetMask, Ip4Address);
pub type Ip4SubnetMask = Ip4Address;
pub type Ip4SubnetMaskBuilder = SimpleBuilder<Ip4SubnetMask>;

impl fmt::Display for Ip4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Eq)]
pub enum Version {
    Ilo2,
    Ilo3,
    Ilo4,
}

pub type VersionBuilder = SimpleBuilder<Version>;
simple_builder_def!(
    Version,
    {
        |value| {
            use Version::*;
            match value.to_ascii_lowercase().as_str() {
                "ilo2" => Ok(Some(Ilo2)),
                "ilo3" => Ok(Some(Ilo3)),
                "ilo4" => Ok(Some(Ilo4)),
                _ => Err(Error::InvalidString {
                    target: "Version",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use Version::*;
            match *value {
                Ilo2 => "iLO2",
                Ilo3 => "iLO3",
                Ilo4 => "iLO4",
            }
            .to_string()
        }
    }
);

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct Route {
    pub dest: Ip4Address,
    pub gateway: Ip4Address,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone, BuilderParse)]
#[ribcl(attributes)]
pub struct FwVersion {
    pub firmware_version: Option<String>,
    pub firmware_date: Option<NaiveDate>,
    pub management_processor: Option<Version>,
    pub license_type: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Copy, Clone)]
pub enum Device {
    Normal,
    Floppy,
    Cdrom,
    Hdd,
    Usb,
    Network(Option<u32>),
}

lazy_static! {
    static ref NETWORK_DEVICE_REGEX: regex::Regex = regex::Regex::new(r"network(\d*)").unwrap();
}

pub type DeviceBuilder = SimpleBuilder<Device>;
simple_builder_def!(
    Device,
    {
        |value| {
            use Device::*;
            match value.to_ascii_lowercase().as_str() {
                "normal" => Ok(Some(Normal)),
                "floppy" => Ok(Some(Floppy)),
                "cdrom" => Ok(Some(Cdrom)),
                "hdd" => Ok(Some(Hdd)),
                "usb" => Ok(Some(Usb)),
                network if NETWORK_DEVICE_REGEX.is_match(network) => {
                    let cnt = NETWORK_DEVICE_REGEX
                        .captures(network)
                        .unwrap()
                        .get(1)
                        .and_then(|m| m.as_str().parse().ok());
                    Ok(Some(Network(cnt)))
                }
                _ => Err(Error::InvalidString {
                    target: "Device",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use Device::*;
            match *value {
                Normal => String::from("NORMAL"),
                Floppy => String::from("FLOPPY"),
                Cdrom => String::from("CDROM"),
                Hdd => String::from("HDD"),
                Usb => String::from("USB"),
                Network(cnt) => {
                    format!("NETWORK{}", cnt.map_or(String::from(""), |c| c.to_string()))
                }
            }
        }
    }
);

impl std::default::Default for DeviceBuilder {
    fn default() -> Self {
        SimpleBuilder(Device::Normal)
    }
}

/*
<GET_PERSISTENT_BOOT
    CDROM = "3" FLOPPY = "6" HDD = "2" USB = "1" NETWORK = "4" NETWORK2 = "5"
    />

<PERSISTENT_BOOT>
    <DEVICE value="USB"/>
    <DEVICE value="CDROM"/>
    <DEVICE value="FLOPPY"/>
    <DEVICE value="HDD"/>
    <DEVICE value="NETWORK1"/>
</PERSISTENT_BOOT>
*/

#[derive(Debug, PartialEq, Serialize)]
pub struct BootDevices(pub Vec<Device>);

#[derive(Default, Debug)]
pub struct BootDevicesBuilder(std::collections::HashMap<u32, Device>);

impl<'a, B> crate::builder_parse::BuilderParse<'_, BootDevicesBuilder> for crate::xml::XmlCursor<B>
where
    B: std::io::BufRead + std::fmt::Debug,
{
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'_>,
        builder: Option<BootDevicesBuilder>,
    ) -> Result<BootDevicesBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(BootDevicesBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref elem) | crate::xml::Event::Empty(ref elem) => elem,
            _ => unreachable!(),
        };
        for attribute in parent_element.attributes() {
            event!(Level::DEBUG, ?attribute);
            let a = attribute?;
            let key = String::from_utf8(a.value.into_owned())
                .map_err::<crate::types::Error, _>(|e| e.into())?;
            let position =
                key.parse::<u32>()
                    .map_err::<crate::types::Error, _>(|error| {
                        crate::types::Error::ParseIntError { value: key, error }
                    })?;
            let value = String::from_utf8(a.key.to_vec())
                .map_err::<crate::types::Error, _>(|e| e.into())?;
            let device = match value.to_ascii_lowercase().ribcl_into() {
                Ok(Some(SimpleBuilder(device))) => device,
                _ => {
                    return Err(crate::types::Error::InvalidString {
                        target: "Device",
                        value,
                    }
                    .into())
                }
            };
            builder.0.insert(position, device);
        }
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event ! (Level :: DEBUG, break_on = ? break_on_value);
        let mut buf = Vec::new();
        if let crate::xml::Event::Start(_) = parent {
            let mut position = 1u32;
            loop {
                let event = self.reader.read_event(&mut buf)?;
                match event.clone() {
                    crate::xml::Event::Start(elem) | crate::xml::Event::Empty(elem) => {
                        event ! (Level :: DEBUG, element = ? elem);
                        let elem_name = String::from_utf8(elem.name().to_vec())?.to_snake_case();
                        match elem_name.as_str() {
                            "ribcl" => {}
                            "response" => {
                                crate::xml::handle_ribcl_response_errors(elem.into_owned())?
                            }
                            "device" => {
                                event!(Level::DEBUG, ?event);
                                let device_builder: DeviceBuilder =
                                    self.builder_parse(event.into_owned().clone(), None)?;
                                event!(Level::DEBUG, ?device_builder);
                                builder.0.insert(position, device_builder.try_into()?);
                                position += 1;
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        event!(Level::DEBUG, "BREAKING {}", "BootDevices");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::DEBUG, "BREAKING {} Eof", "BootDevices");
                        return Err(crate::builder_parse::Error::NotFound {
                            target: "BootDevices",
                        });
                    }
                    _ => {}
                }
                buf.clear();
            }
        }
        Ok(builder)
    }
}

impl std::convert::TryFrom<crate::types::BootDevicesBuilder> for BootDevices {
    type Error = crate::builder_parse::Error;
    fn try_from(builder: crate::types::BootDevicesBuilder) -> Result<BootDevices, Self::Error> {
        event!(Level::DEBUG, ?builder);
        let mut devices: Vec<Device> = vec![];
        for n in builder.0.keys().sorted() {
            devices.push(builder.0[n]);
        }
        Ok(crate::types::BootDevices(devices))
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Status {
    Ok,
    NotInstalled,
    NotPresentOrNotInstalled,
    Other,
    Unknown,
    NotApplicable,
    DiscoveryPending,
    Disabled,
}

pub type StatusBuilder = SimpleBuilder<Status>;
simple_builder_def!(
    Status,
    {
        |value| match value.to_ascii_lowercase().as_str() {
            "ok" => Ok(Some(Status::Ok)),
            "not installed" => Ok(Some(Status::NotInstalled)),
            "not present/not installed" => Ok(Some(Status::NotPresentOrNotInstalled)),
            "other" => Ok(Some(Status::Other)),
            "unknown" => Ok(Some(Status::Unknown)),
            "n/a" => Ok(Some(Status::NotApplicable)),
            "discovery pending" => Ok(Some(Status::DiscoveryPending)),
            "disabled" => Ok(Some(Status::Disabled)),
            _ => Err(Error::InvalidString {
                target: "Status",
                value,
            }),
        }
    },
    {
        |value| {
            match *value {
                Status::Ok => "Ok",
                Status::NotInstalled => "Not Installed",
                Status::NotPresentOrNotInstalled => "Not Present/Not Installed",
                Status::Other => "Other",
                Status::Unknown => "Unknown",
                Status::NotApplicable => "N/A",
                Status::DiscoveryPending => "Discovery Pending",
                Status::Disabled => "Disabled",
            }
            .to_string()
        }
    }
);

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct UnitValue {
    pub value: u32,
    pub unit: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum UidMode {
    On,
    Off,
    Flashing,
}

pub type UidModeBuilder = SimpleBuilder<UidMode>;
simple_builder_def!(
    UidMode,
    {
        |value| {
            use UidMode::*;
            Ok(Some(match value.to_ascii_lowercase().as_str() {
                "on" => On,
                "off" => Off,
                "flashing" => Flashing,
                _ => {
                    return Err(Error::InvalidString {
                        target: "UidMode",
                        value,
                    })
                }
            }))
        }
    },
    {
        |value| {
            use UidMode::*;
            match *value {
                On => "ON",
                Off => "OFF",
                Flashing => "FLASHING",
            }
            .to_string()
        }
    }
);
