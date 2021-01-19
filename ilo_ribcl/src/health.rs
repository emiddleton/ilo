use crate::{
    builder_parse::VecBuilder,
    client,
    ribcl_into::RibclInto,
    types::{
        Ip4Address, Ip4AddressBuilder, MacAddress, MacAddressBuilder, Status, StatusBuilder,
        StringBuilder, U32Builder, UidMode, UidModeBuilder, UnitValue, UnitValueBuilder,
    },
};
use ilo_ribcl_derive::BuilderParse;
use inflector::Inflector;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::{convert::TryInto, io::BufRead};
use tracing::{event, Level};

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
pub struct Fan {
    pub label: Option<String>,
    pub zone: Option<String>,
    pub status: Option<Status>,
    pub speed: Option<UnitValue>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
pub struct Temperature {
    pub label: Option<String>,
    pub location: Option<String>,
    pub status: Option<String>,
    #[ribcl(map = "currentreading")]
    pub current_reading: Option<UnitValue>,
    pub caution: Option<UnitValue>,
    pub critical: Option<UnitValue>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Vrm {}

#[skip_serializing_none]
#[derive(Debug, BuilderParse, Default, Serialize, PartialEq)]
pub struct PowerSupply {
    pub label: Option<String>,
    pub present: Option<String>,
    pub status: Option<String>,
    pub pds: Option<String>,
    pub hotplug_capable: Option<String>,
    pub model: Option<String>,
    pub spare: Option<String>,
    pub serial_number: Option<String>,
    pub capacity: Option<String>,
    pub firmware_version: Option<String>,
}

#[derive(Debug, Serialize, BuilderParse)]
pub struct PowerSupplySummary {
    pub present_power_reading: String,
    pub power_management_controller_firmware_version: String,
    pub power_system_redundancy: String,
    pub hp_power_discovery_services_redundancy_status: String,
    pub high_efficiency_mode: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, BuilderParse)]
pub struct PowerSupplies {
    pub power_supply_summary: Option<PowerSupplySummary>,
    #[ribcl(inline)]
    pub supply: Vec<PowerSupply>,
}

#[derive(Debug, Serialize, PartialEq)] //, BuilderParse)]
pub struct DriveBay {
    pub bay: u32,
    pub product_id: String,
    pub drive_status: String,
    pub uid_led: UidMode,
}

#[derive(Default, Debug)]
pub struct DriveBayBuilder {
    bay: Option<U32Builder>,
    product_id: Option<StringBuilder>,
    drive_status: Option<StringBuilder>,
    uid_led: Option<UidModeBuilder>,
}

impl std::convert::TryFrom<DriveBayBuilder> for DriveBay {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument]
    fn try_from(builder: DriveBayBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(DriveBay));
        Ok(DriveBay {
            bay: {
                event!(Level::DEBUG, bay = "bay", "required field");
                builder
                    .bay
                    .ok_or(crate::builder_parse::Error::NotFound { target: "bay" })?
                    .try_into()?
            },
            product_id: {
                event!(Level::DEBUG, product_id = "product_id", "required field");
                builder
                    .product_id
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "product_id",
                    })?
                    .try_into()?
            },
            drive_status: {
                event!(
                    Level::DEBUG,
                    drive_status = "drive_status",
                    "required field"
                );
                builder
                    .drive_status
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "drive_status",
                    })?
                    .try_into()?
            },
            uid_led: {
                event!(Level::DEBUG, uid_led = "uid_led", "required field");
                builder
                    .uid_led
                    .ok_or(crate::builder_parse::Error::NotFound { target: "uid_led" })?
                    .try_into()?
            },
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Backplane {
    pub firmware_version: String,
    pub enclosure_addr: u32,
    pub drive_bays: Vec<DriveBay>,
}

#[derive(Default, Debug)]
pub struct BackplaneBuilder {
    firmware_version: Option<StringBuilder>,
    enclosure_addr: Option<U32Builder>,
    drive_bays: VecBuilder<DriveBayBuilder>,
}

impl<'a, B: BufRead + std::fmt::Debug> crate::builder_parse::BuilderParse<'a, BackplaneBuilder>
    for crate::xml::XmlCursor<B>
{
    #[tracing::instrument(skip(self, parent))]
    fn builder_parse(
        &mut self,
        parent: crate::xml::Event<'a>,
        builder: ::std::option::Option<BackplaneBuilder>,
    ) -> ::std::result::Result<BackplaneBuilder, crate::builder_parse::Error> {
        let mut builder = builder.unwrap_or_else(BackplaneBuilder::default);
        let parent_element = match parent {
            crate::xml::Event::Start(ref element) | crate::xml::Event::Empty(ref element) => {
                element.clone().into_owned()
            }
            _ => unreachable!(),
        };
        let break_on = parent_element.name().to_ascii_lowercase();
        let break_on_value = String::from_utf8(break_on.clone()).unwrap();
        event!(Level::DEBUG, break_on=?break_on_value);
        let mut buf = Vec::new();
        let mut drive_bay_builder = DriveBayBuilder::default();
        if let crate::xml::Event::Start(_) = parent {
            loop {
                let event = self.reader.read_event(&mut buf)?;
                event!(Level::DEBUG, ?event, ?drive_bay_builder);
                match event.clone() {
                    crate::xml::Event::Start(element) | crate::xml::Event::Empty(element) => {
                        event!(Level::DEBUG, ?element);
                        let elem_name = String::from_utf8(element.name().to_vec())?.to_snake_case();
                        match elem_name.as_str() {
                            "ribcl" => {}
                            "response" => {
                                crate::xml::handle_ribcl_response_errors(element.into_owned())?
                            }
                            "firmware" => {
                                builder.firmware_version = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.firmware_version,
                                )?)
                            }
                            "enclosure" => {
                                builder.enclosure_addr = Some(self.builder_parse(
                                    event.into_owned().clone(),
                                    builder.enclosure_addr,
                                )?)
                            }
                            "drive" => {
                                match drive_bay_builder {
                                    DriveBayBuilder {
                                        bay: None,
                                        product_id: None,
                                        drive_status: None,
                                        uid_led: None,
                                    } => {}
                                    _ => {
                                        builder.drive_bays.0.push(drive_bay_builder);
                                        drive_bay_builder = DriveBayBuilder::default();
                                    }
                                }
                                drive_bay_builder.bay =
                                    Some(self.builder_parse(event.into_owned().clone(), None)?);
                            }
                            "product" => {
                                drive_bay_builder.product_id =
                                    Some(self.builder_parse(event.into_owned().clone(), None)?);
                            }
                            "drive_status" => {
                                drive_bay_builder.drive_status =
                                    Some(self.builder_parse(event.into_owned().clone(), None)?);
                            }
                            "uid" => {
                                drive_bay_builder.uid_led =
                                    Some(self.builder_parse(event.into_owned().clone(), None)?);
                            }
                            ignored => event!(Level::DEBUG, "IGNORED: {:#x?}", ignored),
                        }
                    }
                    crate::xml::Event::End(ref element)
                        if element.name().to_ascii_lowercase() == break_on =>
                    {
                        match drive_bay_builder {
                            DriveBayBuilder {
                                bay: None,
                                product_id: None,
                                drive_status: None,
                                uid_led: None,
                            } => {}
                            _ => {
                                builder.drive_bays.0.push(drive_bay_builder);
                            }
                        }
                        event!(Level::DEBUG, "BREAKING {}", "Backplane");
                        break;
                    }
                    crate::xml::Event::Eof => {
                        event!(Level::DEBUG, "BREAKING {} Eof", "Backplane");
                        return Err(crate::builder_parse::Error::NotFound {
                            target: "Backplane",
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

impl std::convert::TryFrom<BackplaneBuilder> for Backplane {
    type Error = crate::builder_parse::Error;
    #[tracing::instrument(skip(builder))]
    fn try_from(builder: BackplaneBuilder) -> Result<Self, Self::Error> {
        event!(Level::DEBUG, entering_type = stringify!(Backplane));
        Ok(Backplane {
            firmware_version: {
                event!(
                    Level::DEBUG,
                    firmware_version = "firmware_version",
                    "required field"
                );
                builder
                    .firmware_version
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "firmware_version",
                    })?
                    .try_into()?
            },
            enclosure_addr: {
                event!(
                    Level::DEBUG,
                    enclosure_addr = "enclosure_addr",
                    "required field"
                );
                builder
                    .enclosure_addr
                    .ok_or(crate::builder_parse::Error::NotFound {
                        target: "enclosure_addr",
                    })?
                    .try_into()?
            },
            drive_bays: {
                event!(Level::DEBUG, drive_bays = "drive_bays", "vec field");
                event !
                   (Level :: DEBUG, builder_value = ? builder . drive_bays);
                builder.drive_bays.try_into()?
            },
        })
    }
}

#[derive(Debug, Serialize, BuilderParse)]
pub struct Processor {
    pub label: String,
    pub name: String,
    pub status: String,
    pub speed: String,
    pub execution_technology: String,
    pub memory_technology: String,
    pub internal_l1_cache: String,
    pub internal_l2_cache: String,
    pub internal_l3_cache: String,
}

#[derive(Debug, Serialize, BuilderParse)]
pub struct MemoryComponent {
    #[ribcl(map = "memory_location")]
    pub location: String,
    #[ribcl(map = "memory_size")]
    pub size: String,
    #[ribcl(map = "memory_speed")]
    pub speed: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, BuilderParse)]
pub struct Nic {
    pub network_port: String,
    pub port_description: String,
    pub location: String,
    pub mac_address: MacAddress,
    pub ip_address: Option<Ip4Address>,
    pub status: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct StatusRedundancy {
    pub status: Status,
    pub redundancy: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
pub struct HealthAtAGlance {
    pub bios_hardware: Option<Status>,
    pub fans: StatusRedundancy,
    pub temperature: StatusRedundancy,
    pub power_supplies: StatusRedundancy,
    pub drive: Option<Status>,
    pub processor: Option<Status>,
    pub memory: Option<Status>,
    pub network: Option<Status>,
    pub storage: Option<Status>,
    pub vrm: Option<Status>,
}

#[derive(Debug, Serialize, BuilderParse)]
pub struct EmbeddedHealthData {
    pub fans: Vec<Fan>,
    pub temperature: Vec<Temperature>,
    //pub vrms: Vec<Vrm>,
    pub power_supplies: PowerSupplies,
    pub drives: Vec<Backplane>,
    pub processors: Vec<Processor>,
    #[ribcl(map = "memory_components")]
    pub memory: Vec<MemoryComponent>,
    pub nic_information: Vec<Nic>,
    pub health_at_a_glance: HealthAtAGlance,
}

impl client::Node {
    get_method! {
        /// this is documentation for get_embedded_health
        server_info.get_embedded_health -> "get_embedded_health_data" : EmbeddedHealthData,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.10",
        (Ilo4),(Ilo3),(Ilo2,"1.10")
    }
}
