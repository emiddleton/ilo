//#![warn(missing_docs)]

#[macro_use]
pub mod client;
pub mod builder_parse;
pub mod into_ribcl;
pub mod ribcl_into;

pub mod write_ribcl;
#[macro_use]
pub mod xml;

#[macro_use]
pub mod types;
#[macro_use]
pub mod commands;
pub mod cli_helpers;

pub mod ahs;
pub mod authentication;
pub mod boot;
pub mod firmware;
pub mod general;
pub mod health;
pub mod keyboard_mouse;
pub mod license;
pub mod logs;
pub mod network;
pub mod power;
pub mod security;
pub mod snmp;
pub mod virtual_media;
