use crate::{
    builder_parse::VecBuilder,
    client,
    ribcl_into::RibclInto,
    types::{
        Error, F64Builder, Minutes, MinutesBuilder, SimpleBuilder, StringBuilder, U32Builder,
        UnitValue, UnitValueBuilder,
    },
    write_ribcl,
};
use ilo_ribcl_derive::BuilderParse;
use inflector::Inflector;
use lazy_static::lazy_static;
use serde::Serialize;
use std::{convert::TryInto, io::BufRead, str};
use tracing::{event, Level};

#[derive(Debug, Serialize, PartialEq)]
pub enum PowerStatus {
    On,
    Off,
}

pub type PowerStatusBuilder = SimpleBuilder<PowerStatus>;
simple_builder_def!(
    PowerStatus,
    {
        |value| {
            use PowerStatus::*;
            match value.to_ascii_lowercase().as_str() {
                "on" => Ok(Some(On)),
                "off" => Ok(Some(Off)),
                _ => Err(Error::InvalidString {
                    target: "PowerStatus",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use PowerStatus::*;
            match *value {
                On => "Yes",
                Off => "No",
            }
            .to_string()
        }
    }
);

#[derive(Debug, Serialize, PartialEq)]
pub enum PowerOnDelay {
    Yes,
    No,
    Delay15Seconds,
    Delay30Seconds,
    Delay45Seconds,
    Delay60Seconds,
    Random,
    On,
    Off,
    Restore,
}

pub type PowerOnDelayBuilder = SimpleBuilder<PowerOnDelay>;
simple_builder_def!(
    PowerOnDelay,
    {
        |value| {
            use PowerOnDelay::*;
            Ok(Some(match value.to_ascii_lowercase().as_str() {
                "yes" => Yes,
                "no" => No,
                "15" => Delay15Seconds,
                "30" => Delay30Seconds,
                "45" => Delay45Seconds,
                "60" => Delay60Seconds,
                "random" => Random,
                "on" => On,
                "off" => Off,
                "restore" => Restore,
                _ => {
                    return Err(Error::InvalidString {
                        target: "PowerOnDelay",
                        value,
                    })
                }
            }))
        }
    },
    {
        |value| {
            use PowerOnDelay::*;
            match *value {
                Yes => "Yes",
                No => "No",
                Delay15Seconds => "15",
                Delay30Seconds => "30",
                Delay45Seconds => "45",
                Delay60Seconds => "60",
                Random => "Random",
                On => "On",
                Off => "Off",
                Restore => "Restore",
            }
            .to_string()
        }
    }
);

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
pub struct PowerReadings {
    #[ribcl(map = "present_power_reading")]
    pub present: UnitValue,
    #[ribcl(map = "average_power_reading")]
    pub average: UnitValue,
    #[ribcl(map = "maximum_power_reading")]
    pub maximum: UnitValue,
    #[ribcl(map = "minimum_power_reading")]
    pub minimum: UnitValue,
}

#[derive(Debug, Serialize, PartialEq)] //, BuilderParse)]
pub struct PState {
    pub state: u32,
    pub total_average: f64,
}

#[derive(std::default::Default, std::fmt::Debug)]
pub struct PstateBuilder {
    state: Option<U32Builder>,
    total_average: Option<F64Builder>,
}

impl<'a, B: BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, PstateBuilder>
    for crate::xml::XmlCursor<B>
{
    #[tracing::instrument(skip(parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: ::std::option::Option<PstateBuilder>,
    ) -> ::std::result::Result<PstateBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(PstateBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event ! (Level :: DEBUG, break_on = ? break_on_value);
        let mut buf = Vec::new();
        if let crate::xml::Event::Start(_) = parent {
            loop {
                let event = self.reader.read_event(&mut buf)?;
                match event.clone() {
                    crate::xml::Event::Start(elem) | crate::xml::Event::Empty(elem) => {
                        event!(Level::DEBUG, element=?elem);
                        let elem_name = String::from_utf8(elem.name().to_vec())?.to_snake_case();
                        match elem_name.as_str() {
                            "ribcl" => {}
                            "response" => {
                                crate::xml::handle_ribcl_response_errors(elem.into_owned())?
                            }
                            "total_average" => {
                                builder.total_average = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.total_average,
                                )?)
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        event!(Level::DEBUG, "BREAKING {}", "PState");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::ERROR, "BREAKING {} Eof", "PState");
                        return Err(crate::builder_parse::Error::NotFound { target: "PState" });
                    }
                    _ => {}
                }
                buf.clear();
            }
        }
        Ok(builder)
    }
}

impl std::convert::TryFrom<PstateBuilder> for PState {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument]
    fn try_from(builder: PstateBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(PState));
        Ok(PState {
            state: {
                event!(Level::DEBUG, state = "state", "required field");
                builder
                    .state
                    .ok_or(crate::builder_parse::Error::NotFound { target: "state" })?
                    .try_into()?
            },
            total_average: {
                event!(
                    Level::DEBUG,
                    total_average = "total_average",
                    "required field"
                );
                builder
                    .total_average
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "total_average",
                    })?
                    .try_into()?
            },
        })
    }
}

#[derive(Debug, Serialize, PartialEq)] //, BuilderParse)]
pub struct ProcessorState {
    pub position: u32,
    pub current_pstate: u32,
    pub pstates: Vec<PState>,
}

#[derive(std::default::Default, std::fmt::Debug)]
pub struct ProcessorStateBuilder {
    position: Option<U32Builder>,
    current_pstate: Option<U32Builder>,
    pstates: VecBuilder<PstateBuilder>,
}

lazy_static! {
    static ref PSTATE_REGEX: regex::Regex = regex::Regex::new(r"pstate_(\d*)").unwrap();
}

impl<'a, B: BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, ProcessorStateBuilder>
    for crate::xml::XmlCursor<B>
{
    #[tracing::instrument(skip(parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: ::std::option::Option<ProcessorStateBuilder>,
    ) -> ::std::result::Result<ProcessorStateBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(ProcessorStateBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event ! (Level :: DEBUG, break_on = ? break_on_value);
        let mut buf = Vec::new();
        if let crate::xml::Event::Start(_) = parent {
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
                            "current_pstate" => {
                                builder.current_pstate = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.current_pstate,
                                )?)
                            }
                            pstate if PSTATE_REGEX.is_match(pstate) => {
                                let state: Option<U32Builder> = PSTATE_REGEX
                                    .captures(pstate)
                                    .unwrap()
                                    .get(1)
                                    .and_then(|m| m.as_str().parse().ok().map(SimpleBuilder));
                                let mut pstates = builder.pstates.0;
                                event!(Level::DEBUG, "BEFORE INLINE Vec -> {:#x?}", pstates);
                                let mut pstate_builder: PstateBuilder =
                                    self.builder_parse(event.into_owned().clone(), None)?;
                                pstate_builder.state = state;
                                pstates.push(pstate_builder);
                                event!(Level::DEBUG, "AFTER INLINE Vec -> {:#x?}", pstates);
                                builder.pstates = VecBuilder(pstates)
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        event!(Level::DEBUG, "BREAKING {}", "ProcessorState");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::ERROR, "BREAKING {} Eof", "ProcessorState");
                        return Err(crate::builder_parse::Error::NotFound {
                            target: "ProcessorState",
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

impl std::convert::TryFrom<ProcessorStateBuilder> for ProcessorState {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument]
    fn try_from(builder: ProcessorStateBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(ProcessorState));
        Ok(ProcessorState {
            position: {
                event!(Level::DEBUG, position = "position", "required field");
                builder
                    .position
                    .ok_or(crate::builder_parse::Error::NotFound { target: "position" })?
                    .try_into()?
            },
            current_pstate: {
                event!(
                    Level::DEBUG,
                    current_pstate = "current_pstate",
                    "required field"
                );
                builder
                    .current_pstate
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "current_pstate",
                    })?
                    .try_into()?
            },
            pstates: {
                event!(Level::DEBUG, pstate = "pstates", "vec field");
                event !
                   (Level :: DEBUG, builder_value = ? builder . pstates);
                builder.pstates.try_into()?
            },
        })
    }
}
#[derive(Debug, Serialize, PartialEq)]
pub struct HostPowerRegInfo {
    pub number_processors: u32,
    pub number_pstates: u32,
    pub processor_pstates: Vec<ProcessorState>,
}

lazy_static! {
    static ref PROCESSOR_REGEX: regex::Regex = regex::Regex::new(r"processor_(\d*)").unwrap();
}

#[derive(std::default::Default, std::fmt::Debug)]
pub struct HostPowerRegInfoBuilder {
    number_processors: Option<U32Builder>,
    number_pstates: Option<U32Builder>,
    processor_pstates: VecBuilder<ProcessorStateBuilder>,
}

impl<'a, B: BufRead + std::fmt::Debug>
    crate::builder_parse::BuilderParse<'a, HostPowerRegInfoBuilder> for crate::xml::XmlCursor<B>
{
    //#[tracing::instrument(skip(parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: ::std::option::Option<HostPowerRegInfoBuilder>,
    ) -> ::std::result::Result<HostPowerRegInfoBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(HostPowerRegInfoBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event ! (Level :: DEBUG, break_on = ? break_on_value);
        let mut buf = Vec::new();
        if let crate::xml::Event::Start(_) = parent {
            loop {
                let event = self.reader.read_event(&mut buf)?;
                match event.clone() {
                    crate::xml::Event::Start(elem) | crate::xml::Event::Empty(elem) => {
                        event!(Level::DEBUG, element=?elem);
                        let elem_name = String::from_utf8(elem.name().to_vec())?.to_snake_case();
                        match elem_name.as_str() {
                            "ribcl" => {}
                            "response" => {
                                crate::xml::handle_ribcl_response_errors(elem.into_owned())?
                            }
                            "number_processors" => {
                                builder.number_processors = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.number_processors,
                                )?)
                            }
                            "number_pstates" => {
                                builder.number_pstates = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.number_pstates,
                                )?)
                            }
                            processor if PROCESSOR_REGEX.is_match(processor) => {
                                let position: Option<U32Builder> = PROCESSOR_REGEX
                                    .captures(processor)
                                    .unwrap()
                                    .get(1)
                                    .and_then(|m| m.as_str().parse().ok().map(SimpleBuilder));
                                let mut processor_pstates = builder.processor_pstates.0;
                                event!(
                                    Level::DEBUG,
                                    "BEFORE INLINE Vec -> {:#x?}",
                                    processor_pstates
                                );
                                let mut processor_pstate_builder: ProcessorStateBuilder =
                                    self.builder_parse(event.into_owned().clone(), None)?;
                                processor_pstate_builder.position = position;
                                processor_pstates.push(processor_pstate_builder);
                                event!(
                                    Level::DEBUG,
                                    "AFTER INLINE Vec -> {:#x?}",
                                    processor_pstates
                                );
                                builder.processor_pstates = VecBuilder(processor_pstates)
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        event!(Level::DEBUG, "BREAKING {}", "HostPowerRegInfo");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::ERROR, "BREAKING {} Eof", "HostPowerRegInfo");
                        return Err(crate::builder_parse::Error::NotFound {
                            target: "HostPowerRegInfo",
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

impl std::convert::TryFrom<HostPowerRegInfoBuilder> for HostPowerRegInfo {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument]
    fn try_from(builder: HostPowerRegInfoBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(HostPowerRegInfo));
        Ok(HostPowerRegInfo {
            number_processors: {
                event!(
                    Level::DEBUG,
                    number_processors = "number_processors",
                    "required field"
                );
                builder
                    .number_processors
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "number_processors",
                    })?
                    .try_into()?
            },
            number_pstates: {
                event!(
                    Level::DEBUG,
                    number_pstates = "number_pstates",
                    "required field"
                );
                builder
                    .number_pstates
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "number_pstates",
                    })?
                    .try_into()?
            },
            processor_pstates: {
                event!(
                    Level::DEBUG,
                    processor_pstates = "processor_pstates",
                    "vec field"
                );
                event !
                   (Level :: DEBUG, builder_value = ? builder.processor_pstates);
                builder.processor_pstates.try_into()?
            },
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Copy, Clone)]
pub enum PwrAlert {
    Disabled,
    Peak { threshold: u32, duration: u32 },
    Average { threshold: u32, duration: u32 },
}

impl Default for PwrAlert {
    fn default() -> Self {
        PwrAlert::Disabled
    }
}

#[derive(Debug, Default)]
pub struct PwrAlertBuilder {
    pwr_alert_type: Option<StringBuilder>,
    threshold: Option<U32Builder>,
    duration: Option<U32Builder>,
}

impl<'a, B: BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, PwrAlertBuilder>
    for crate::xml::XmlCursor<B>
{
    #[tracing::instrument(skip(parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: Option<PwrAlertBuilder>,
    ) -> Result<PwrAlertBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(PwrAlertBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        for attribute in parent_element.attributes() {
            match attribute {
                Ok(a) if "type".as_bytes() == a.key.to_ascii_lowercase() => {
                    builder.pwr_alert_type = a.ribcl_into()?
                }
                Ok(a) if "threshold".as_bytes() == a.key.to_ascii_lowercase() => {
                    builder.threshold = a.ribcl_into()?
                }
                Ok(a) if "duration".as_bytes() == a.key.to_ascii_lowercase() => {
                    builder.duration = a.ribcl_into()?
                }
                _ => {}
            }
        }
        Ok(builder)
    }
}

impl std::convert::TryFrom<PwrAlertBuilder> for PwrAlert {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument(skip(builder))]
    fn try_from(builder: PwrAlertBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(Pwreg));
        let pwr_alert_type: Result<String, _> = builder
            .pwr_alert_type
            .ok_or(crate::builder_parse::Error::NotFound {
                target: "pwr_alert_type",
            })?
            .try_into();

        match pwr_alert_type?.to_lowercase().as_str() {
            "disabled" => Ok(PwrAlert::Disabled),
            "peak" => {
                let threshold = builder
                    .threshold
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "threshold",
                    })?
                    .try_into()?;
                let duration = builder
                    .duration
                    .ok_or(crate::builder_parse::Error::NotFound { target: "duration" })?
                    .try_into()?;
                Ok(PwrAlert::Peak {
                    threshold,
                    duration,
                })
            }
            "average" => {
                let threshold = builder
                    .threshold
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "threshold",
                    })?
                    .try_into()?;
                let duration = builder
                    .duration
                    .ok_or(crate::builder_parse::Error::NotFound { target: "duration" })?
                    .try_into()?;
                Ok(PwrAlert::Average {
                    threshold,
                    duration,
                })
            }
            value => Err(Error::InvalidString {
                target: "pwr_alert_type",
                value: value.to_string(),
            })
            .map_err(|e| e.into()),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub enum HostPowerSaverMode {
    Off,
    Min,
    Auto,
    Max,
}

pub type HostPowerSaverModeBuilder = SimpleBuilder<HostPowerSaverMode>;
simple_builder_def!(
    HostPowerSaverMode,
    {
        |value| {
            use HostPowerSaverMode::*;
            match value.to_ascii_lowercase().as_str() {
                "off" => Ok(Some(Off)),
                "min" => Ok(Some(Min)),
                "auto" => Ok(Some(Auto)),
                "max" => Ok(Some(Max)),
                _ => Err(Error::InvalidString {
                    target: "HostPowerSaverMOde",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use HostPowerSaverMode::*;
            match *value {
                Off => "1",
                Min => "2",
                Auto => "3",
                Max => "4",
            }
            .to_string()
        }
    }
);

#[derive(Debug, Serialize, PartialEq)]
pub enum PowerCap {
    Off,
    Value(u32),
}

lazy_static! {
    static ref NUM_REGEX: regex::Regex = regex::Regex::new(r"(\d*)").unwrap();
}

pub type PowerCapBuilder = SimpleBuilder<PowerCap>;
simple_builder_def!(
    PowerCap,
    {
        |value| {
            use PowerCap::*;
            match value.to_ascii_lowercase().replace("\"", "").as_str() {
                "off" => Ok(Some(Off)),
                val if NUM_REGEX.is_match(val) => {
                    match NUM_REGEX
                        .captures(val)
                        .unwrap()
                        .get(1)
                        .map(|m| m.as_str().parse())
                    {
                        Some(Ok(v)) => Ok(Some(Value(v))),
                        _ => Err(Error::InvalidString {
                            target: "PowerCap",
                            value,
                        }),
                    }
                }
                _ => Err(Error::InvalidString {
                    target: "PowerCap",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use PowerCap::*;
            match *value {
                Off => String::from("OFF"),
                Value(cnt) => cnt.to_string(),
            }
        }
    }
);

//simple_builder_alias!(PowerMicroVersion, String);
pub type PowerMicroVersion = String;
pub type PowerMicroVersionBuilder = SimpleBuilder<PowerMicroVersion>;

#[derive(Debug, Serialize, PartialEq)]
pub struct Pwreg {
    pub efficiency_mode: Option<String>,
    pub pwr_alert: PwrAlert,
    pub host_power: Option<PowerStatus>,
}

#[derive(Default, Debug)]
pub struct PwregBuilder {
    efficiency_mode: Option<Option<StringBuilder>>,
    pwr_alert: Option<PwrAlertBuilder>,
    host_power: Option<Option<PowerStatusBuilder>>,
}

impl<'a, B: BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, PwregBuilder>
    for crate::xml::XmlCursor<B>
{
    #[tracing::instrument(skip(parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: ::std::option::Option<PwregBuilder>,
    ) -> ::std::result::Result<PwregBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(PwregBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event ! (Level :: DEBUG, break_on = ? break_on_value);
        let mut buf = Vec::new();
        if let crate::xml::Event::Start(_) = parent {
            loop {
                let event = self.reader.read_event(&mut buf)?;
                match event.clone() {
                    crate::xml::Event::Start(elem) | crate::xml::Event::Empty(elem) => {
                        event ! (Level :: DEBUG, element = ? elem);
                        match str::from_utf8(&elem.name().to_ascii_lowercase())? {
                            "ribcl" => {}
                            "response" => {
                                crate::xml::handle_ribcl_response_errors(elem.into_owned())?
                            }
                            "efficiency_mode" => {
                                builder.efficiency_mode = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.efficiency_mode,
                                )?)
                            }
                            "pwralert" => {
                                builder.pwr_alert =
                                    Some(self.builder_parse(
                                        event.into_owned().clone(),
                                        builder.pwr_alert,
                                    )?)
                            }
                            "get_host_power" => {
                                builder.host_power = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.host_power,
                                )?)
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        event!(Level::DEBUG, "BREAKING {}", "Pwreg");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::ERROR, "BREAKING {} Eof", "Pwreg");
                        return Err(crate::builder_parse::Error::NotFound { target: "Pwreg" });
                    }
                    _ => {}
                }
                buf.clear();
            }
        }
        Ok(builder)
    }
}

impl std::convert::TryFrom<PwregBuilder> for Pwreg {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument(skip(builder))]
    fn try_from(builder: PwregBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(Pwreg));
        Ok(Pwreg {
            efficiency_mode: {
                event!(
                    Level::DEBUG,
                    efficiency_mode = "efficiency_mode",
                    "optional field"
                );
                event !
                   (Level :: DEBUG, builder_value = ? builder .
                    efficiency_mode);
                builder
                    .efficiency_mode
                    .unwrap_or_default()
                    .map_or_else::<Result<_, crate::builder_parse::Error>, _, _>(
                        || Ok(None),
                        |b| match b.try_into() {
                            Ok(val) => Ok(Some(val)),
                            Err(crate::builder_parse::Error::NotFound { .. }) => Ok(None),
                            Err(e) => Err(e),
                        },
                    )?
            },
            pwr_alert: {
                event!(Level::DEBUG, pwr_alert = "pwr_alert", "required field");
                builder
                    .pwr_alert
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "pwr_alert",
                    })?
                    .try_into()?
            },
            host_power: {
                event!(Level::DEBUG, host_power = "host_power", "optional field");
                event!(Level::DEBUG, builder_value=?builder.host_power);
                builder
                    .host_power
                    .unwrap_or_default()
                    .map_or_else::<Result<_, crate::builder_parse::Error>, _, _>(
                        || Ok(None),
                        |b| match b.try_into() {
                            Ok(val) => Ok(Some(val)),
                            Err(crate::builder_parse::Error::NotFound { .. }) => Ok(None),
                            Err(e) => Err(e),
                        },
                    )?
            },
        })
    }
}

impl write_ribcl::WriteRibcl for Pwreg {
    fn write_ribcl<W: std::fmt::Write>(&self, writer: &mut W) -> Result<(), write_ribcl::Error> {
        match *self {
            Pwreg {
                pwr_alert: PwrAlert::Disabled,
                ..
            } => {
                write!(writer, "<pwralert type=\"DISABLED\"/>")?;
            }
            Pwreg {
                pwr_alert:
                    PwrAlert::Peak {
                        threshold,
                        duration,
                    },
                ..
            } => {
                write!(
                    writer,
                    "<pwralert type=\"PEAK\"/><pwralert_settings threshold=\"{}\" duration=\"{}\"/>",
                    &threshold, &duration
                )?;
            }
            Pwreg {
                pwr_alert:
                    PwrAlert::Average {
                        threshold,
                        duration,
                    },
                ..
            } => {
                write!(
                    writer,
                    "<pwralert type=\"AVERAGE\"/><pwralert_settings threshold=\"{}\" duration=\"{}\"/>",
                    &threshold, &duration
                )?;
            }
        }
        Ok(())
    }
}

impl client::Node {
    mod_method!(
        /// Simulates a physical press of the server power button
        server_info.press_pwr_btn
    );

    // mod_method!(
    //    /// Will power on the server if toggle is true set to yes otherwise will power off/leave off.
    //    server_info.hold_pwr_btn_toggle : "hold_pwr_btn" ("toggle" : bool),
    //    "iLO 4 or iLO 2", (Ilo4), (Ilo2)
    // );
    //
    mod_method!(
        /// Will power off the server if on
        server_info.hold_pwr_btn
    );

    get_method!(
        /// Returns the power state of the server i.e. if power is on or not
        server_info.get_host_power_status -> "get_host_power" : PowerStatus
    );

    get_method!(
        /// Returns the iLO power regulator info
        server_info.get_host_power_reg_info -> HostPowerRegInfo,
        "iLO 2 version >= 1.10",
        (Ilo2,"1.10")
    );

    mod_method!(
        /// Turn on/off the server gracefully using ACPI Power button functionality.
        ///
        /// # Examples
        ///
        /// ```
        /// node.set_host_power(PowerStatus::Off).await?;
        /// ```
        ///
        server_info.set_host_power("host_power": PowerStatus)
    );

    get_method!(
        /// Returns the servers automatic power on and power on delay settings.
        server_info.get_server_auto_pwr -> "^server_auto_pwr" : PowerOnDelay,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.20",
        (Ilo4),(Ilo3),(Ilo2,"1.20")
    );

    mod_method!(
        /// Set server automatic power on and delay settings
        server_info.server_auto_pwr("value": PowerOnDelay),
        "iLO 4 or iLO 3 or iLO 2 version >= 1.20",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.20")
    );

    get_method!(
        /// Returns the servers power readings from power supply.
        server_info.get_power_readings -> PowerReadings
    );

    get_method!(
        /// Returns the servers power on time in minutes.
        server_info.get_server_power_on_time -> "server_power_on_minutes": Minutes
    );

    mod_method!(
        /// Clears the server power on time
        server_info.clear_server_power_on_time,
        "iLO 4 or iLO 3 or iLO 2 version >= 2.00",
        (Ilo4),
        (Ilo3),
        (Ilo2, "2.00")
    );

    get_method!(
        /// Returns the state of the servers processor power regulator.
        server_info.get_host_power_saver_status -> "get_host_power_saver": HostPowerSaverMode,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.10",
        (Ilo4),(Ilo3),(Ilo2,"1.10")
    );

    mod_method!(
        /// Set the configuration of the servers processor power regulator.
        server_info.set_host_power_saver("host_power_saver": HostPowerSaverMode),
        "iLO 4 or iLO 3 or iLO 2 version >= 1.10",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.10")
    );

    get_method!(
        /// Returns the server power cap.
        server_info.get_power_cap -> "^power_cap" : PowerCap,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.30")
    );

    mod_method!(
        /// Sets the servers power cap.
        server_info.set_power_cap("power_cap": PowerCap),
        "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.30")
    );

    get_method!(
        /// Returns the host power micro version
        server_info.get_host_pwr_micro_ver -> "^pwr_micro$" : PowerMicroVersion
    );

    get_method!(
        /// get power alert threshold settings
        server_info.get_pwreg -> Pwreg,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.70",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.70")
    );

    // TODO: add parse multiple arguments
    // server_info.set_pwreg("type":, "threshold":, "duration":)
    mod_method!(
        /// set power alert threshold
        server_info.set_pwreg(Pwreg),
        "iLO 4 or iLO 3 or iLO 2 version >= 1.70",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.70")
    );
}
