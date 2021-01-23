use crate::{
    builder_parse::VecBuilder,
    client,
    into_ribcl::IntoRibcl,
    ribcl_into::RibclInto,
    types::{
        BoolBuilder, Port, PortBuilder, SimpleBuilder, StringBuilder, U32Builder, UidMode,
        UidModeBuilder, Url, UrlBuilder,
    },
    write_ribcl,
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::{convert::TryInto, fmt};

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct SmbiosField {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SmbiosData(String); //Vec<u8>);

impl fmt::Display for SmbiosData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x?}", self.0)
    }
}

pub type SmbiosDataBuilder = SimpleBuilder<SmbiosData>;
simple_builder_def!(
    SmbiosData,
    {
        |value| Ok(Some(SmbiosData(value)))
        /*
            |value| match base64::decode(value.to_ascii_lowercase().as_str()) {
            Ok(v) => Ok(Some(SmbiosData(v))),
            Err(e) => {
                println!("{:#x?}", e);
                Err(Error::InvalidString {
                    target: "SmbiosData",
                    value,
                })
            }
        }
        */
    },
    { |value: &SmbiosData| format!("{:x?}", value.0) }
);

#[derive(Debug, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct SmbiosRecord {
    #[ribcl(map = "type")]
    pub smbios_type: u32,
    #[ribcl(map = "b64_data")]
    pub data: SmbiosData,
    #[ribcl(inline, elements, map = "field")]
    pub fields: Vec<SmbiosField>,
}

#[skip_serializing_none]
#[derive(WriteRibcl, Debug, Default, PartialEq, Serialize, BuilderParse)]
pub struct GlobalSettings {
    pub session_timeout: Option<u32>,
    pub f8_prompt_enabled: Option<bool>,
    pub http_port: Option<Port>,
    pub https_port: Option<Port>,
    pub remote_console_port: Option<Port>,
    pub min_password: Option<u32>,
    pub ilo_funct_enabled: Option<bool>,
    pub virtual_media_port: Option<Port>,
    pub f8_login_required: Option<bool>,
    pub enforce_aes: Option<bool>,
    pub authentication_failure_logging: Option<String>,
    pub ssh_port: Option<Port>,
    pub ssh_status: Option<bool>,
    pub serial_cli_status: Option<String>,
    pub serial_cli_speed: Option<u32>,
    pub rbsu_post_ip: Option<bool>,
    // ilo2
    pub remote_console_port_status: Option<u32>,
    pub remote_console_encryption: Option<bool>,
    pub remote_keyboard_model: Option<String>,
    pub passthrough_config: Option<u32>,
    pub terminal_services_port: Option<u32>,
    pub telnet_enabled: Option<u32>,
    pub high_performance_mouse: Option<String>,
    pub console_capture_enable: Option<bool>,
    pub console_capture_boot_buffer_enable: Option<bool>,
    pub console_capture_fault_buffer_enable: Option<bool>,
    pub interactive_console_replay_enable: Option<bool>,
    pub console_capture_port: Option<Port>,
    pub capture_auto_export_enable: Option<bool>,
    pub capture_auto_export_location: Option<Url>,
    pub capture_auto_export_username: Option<String>,
    pub capture_auto_export_password: Option<String>,
    pub shared_console_enable: Option<bool>,
    pub shared_console_port: Option<Port>,
    pub key_up_key_down: Option<bool>,
    pub capture_manual_export: Option<bool>,
    pub remote_console_acquire: Option<bool>,
    pub vsp_software_flow_control: Option<bool>,
    pub rawvsp_port: Option<Port>,
}

//simple_builder_alias!(ServerName, String);
pub type ServerName = String;
pub type ServerNameBuilder = SimpleBuilder<ServerName>;

#[derive(Debug, Default, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct Language {
    pub lang_id: String, // {"EN"}
    pub language: String,
}

impl write_ribcl::WriteRibcl for Language {
    fn write_ribcl<W: std::fmt::Write>(&self, writer: &mut W) -> Result<(), write_ribcl::Error> {
        write!(writer, "<set_language lang_id=\"{}\"/>", self.lang_id)?;
        Ok(())
    }
}

impl client::Node {
    mod_method!(
        /// Reset the iLO to factory default settings
        rib_info.factory_defaults
    );

    get_method!(
        /// Returns host data
        server_info.get_host_data -> Vec<SmbiosRecord>
    );

    get_method!(
        /// Returns the global settings
        rib_info.get_global_settings -> GlobalSettings
    );

    mod_method!(
        /// Update the global settings
        rib_info.mod_global_settings(GlobalSettings)
    );

    get_method!(
        /// Returns the server name
        server_info.get_server_name -> "server_name": ServerName,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
        (Ilo4),(Ilo3),(Ilo2,"1.20")
    );

    mod_method!(
        /// Update the server name
        server_info.server_name("value": ServerName),
        "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.30")
    );

    /*
    get_method!(
        bladesystem_info.get_oa_info -> ***,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.30")
    );

    get_method!(
        rack_info.get_enclosure_ip -> ***,
        "iLO 4 or iLO 3 or iLO 2 version >= 1.10",
        (Ilo4),
        (Ilo3),
        (Ilo2, "1.10")
    );
    */

    get_method!(
        /// Returns the server UID status
        server_info.get_uid_status -> UidMode
    );

    mod_method!(
        /// Update the server UID status
        server_info.uid_control("uid": UidMode)
    );

    get_method!(
        /// Returns all supported languages
        rib_info.get_all_languages -> "get_all_languages" : Vec<Language>,
        "iL0 4 or iL0 3 version >= 1.20",
        (Ilo4),(Ilo3, "1.20")
    );
    /*
    pub async fn get_all_languages(&mut self) -> Result<Vec<types::Language>, commands::Error> {
        assert_fw!(
            self.firmware(),
            "iL0 4 or iL0 3 version >= 1.20",
            (Ilo4),
            (Ilo3, "1.20")
        );
        let mut request = String::new();
        ribcl_command!(request, self.auth(), rib_info, read, get_all_languages);
        let response = self.send(request.into_bytes()).await?;
        Ok(ribcl_parse_response!(response, "get_all_languages" -> Vec<types::Language>)??)
    }
    */

    get_method!(
        /// Returns the selected language
        rib_info.get_language -> Language,
        "iL0 4 or iL0 3 version >= 1.20",
        (Ilo4),
        (Ilo3, "1.20")
    );

    mod_method!(
        /// Update the language
        rib_info.set_language(Language),
        "iL0 4 or iL0 3 version >= 1.20",
        (Ilo4),
        (Ilo3, "1.20")
    );
}
