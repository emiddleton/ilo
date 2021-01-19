use crate::{
    client,
    into_ribcl::IntoRibcl,
    types::{
        BoolBuilder, DomainName, DomainNameBuilder, HostName, HostNameBuilder, Ip4Address,
        Ip4AddressBuilder, Ip4SubnetMask, Ip4SubnetMaskBuilder, MacAddress, MacAddressBuilder,
        Route, RouteBuilder, Timezone, TimezoneBuilder, U32Builder,
    },
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[skip_serializing_none]
#[derive(Debug, Default, PartialEq, Serialize, BuilderParse, WriteRibcl)]
pub struct NetworkSettings {
    pub enable_nic: Option<bool>,
    pub shared_network_port: Option<bool>,
    #[ribcl(map = "vlan.enabled")]
    pub vlan_enabled: Option<bool>,
    pub vlan_id: Option<u32>,

    pub speed_autoselect: Option<bool>,
    pub nic_speed: Option<u32>,
    pub full_duplex: Option<bool>,
    pub dhcp_enable: Option<bool>,
    pub dhcp_gateway: Option<bool>,
    pub dhcp_dns_server: Option<bool>,
    pub dhcp_wins_server: Option<bool>,
    pub dhcp_static_route: Option<bool>,
    pub dhcp_domain_name: Option<bool>,
    pub reg_wins_server: Option<bool>,
    pub reg_ddns_server: Option<bool>,
    pub ping_gateway: Option<bool>,
    pub gratuitous_arp: Option<bool>,
    pub mac_address: Option<MacAddress>,
    pub ip_address: Option<Ip4Address>,
    pub subnet_mask: Option<Ip4SubnetMask>,
    pub gateway_ip_address: Option<Ip4Address>,
    pub dns_name: Option<HostName>,
    pub domain_name: Option<DomainName>,
    pub prim_dns_server: Option<Ip4Address>,
    pub sec_dns_server: Option<Ip4Address>,
    pub ter_dns_server: Option<Ip4Address>,
    pub prim_wins_server: Option<Ip4Address>,
    pub sec_wins_server: Option<Ip4Address>,

    #[ribcl(map = "{dest,gateway}")]
    pub static_route_1: Option<Route>,
    #[ribcl(map = "{dest,gateway}")]
    pub static_route_2: Option<Route>,
    #[ribcl(map = "{dest,gateway}")]
    pub static_route_3: Option<Route>,

    #[cfg(any(feature = "ilo3", feature = "ilo4"))]
    pub dhcp_sntp_settings: Option<bool>,
    #[cfg(any(feature = "ilo3", feature = "ilo4"))]
    #[ribcl(map = "sntp_server1")]
    pub sntp_server_1: Option<Ip4Address>,
    #[cfg(any(feature = "ilo3", feature = "ilo4"))]
    #[ribcl(map = "sntp_server2")]
    pub sntp_server_2: Option<Ip4Address>,
    #[cfg(any(feature = "ilo3", feature = "ilo4"))]
    pub timezone: Option<Timezone>,

    #[cfg(any(feature = "ilo2", feature = "ilo3", feature = "ilo4"))]
    pub enclosure_ip_enable: Option<bool>,
}

impl client::Node {
    get_method!(
        /// get network settings
        rib_info.get_network_settings -> NetworkSettings
    );

    mod_method!(
        /// set network settings
        rib_info.mod_network_settings(NetworkSettings)
    );
}
