use crate::{
    client,
    into_ribcl::IntoRibcl,
    ribcl_into::RibclInto,
    types::{BoolBuilder, Ip4Address, Ip4AddressBuilder, StringBuilder, U32Builder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct TrapCommunity {
    pub version: String,
    pub value: String,
}

#[skip_serializing_none]
#[derive(Debug, Default, WriteRibcl, PartialEq, Serialize, BuilderParse)]
pub struct SnmpImSettings {
    // ilo2
    pub snmp_address_1: Option<Ip4Address>,
    pub snmp_address_2: Option<Ip4Address>,
    pub snmp_address_3: Option<Ip4Address>,
    pub rib_traps: Option<bool>,
    pub os_traps: Option<bool>,
    pub snmp_passthrough_status: Option<bool>,
    pub web_agent_ip_address: Option<String>,
    pub cim_security_mask: Option<u32>,
    // ilo3, ilo4
    pub snmp_address_1_recommunity: Option<String>,

    /* TODO: sort out proc-macro crash
    #[ribcl(map = "{version,value}")]
    pub snmp_address_1_trapcommunity: Option<TrapCommunity>,
    pub snmp_address_2_recommunity: Option<String>,
    #[ribcl(map = "{version,value}")]
    pub snmp_address_2_trapcommunity: Option<TrapCommunity>,
    pub snmp_address_3_recommunity: Option<String>,
    #[ribcl(map = "{version,value}")]
    pub snmp_address_3_trapcommunity: Option<TrapCommunity>,
    */
    pub agentless_management_enable: Option<bool>,
    pub snmp_sys_contact: Option<String>,
    pub snmp_sys_location: Option<String>,
    pub snmp_system_role: Option<String>,
    pub snmp_system_role_detail: Option<String>,
    pub cold_start_trap_broadcast: Option<bool>,
}

impl client::Node {
    get_method!(
        /// get snmp im settings
        rib_info.get_snmp_im_settings -> SnmpImSettings
    );

    mod_method!(
        /// set snmp im settings
        rib_info.mod_snmp_im_settings(SnmpImSettings),
        "iLO 4 or iLO 3 version >= 1.20 or ilO 2",
        (Ilo4),
        (Ilo3, "1.20"),
        (Ilo2)
    );
}
