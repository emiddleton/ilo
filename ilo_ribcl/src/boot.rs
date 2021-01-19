use crate::{
    client, commands,
    into_ribcl::IntoRibcl,
    types::{BootDevices, BootDevicesBuilder, Device, DeviceBuilder},
};

impl client::Node {
    mod_method!(
        /// Reset the iLO board
        rib_info.reset_rib
    );

    mod_method!(
        /// Power cycle the server without graceful shutdown, for graceful ACPI based reboot
        /// use [Node::set_host_power]
        server_info.reset_server
    );

    mod_method!(
        /// Forces a cold reboot of the server if it is running
        server_info.cold_boot_server
    );

    mod_method!(
        /// Forces a warm reboot of the server if it is running
        server_info.warm_boot_server
    );

    get_method!(
        /// Get the one time boot device
        server_info.get_one_time_boot -> "boot_type": Device,
        "iLO 4 or iLO 3 or iLO 2 version >= 2.06",
        (Ilo4),
        (Ilo3),
        (Ilo2,"2.06")
    );

    mod_method!(
        /// Set the one time boot device
        server_info.set_one_time_boot("value": Device),
        "iLO 4 or iLO 3 or iLO 2 version >= 2.06",
        (Ilo4),
        (Ilo3),
        (Ilo2, "2.06")
    );

    get_method!(
        /// get persistent boot
        server_info.get_persistent_boot -> "(get_)?persistent_boot" : BootDevices,
        "iLO 4 or iLO 3 or iLO 2 version >= 2.06",
        (Ilo4),
        (Ilo3),
        (Ilo2, "2.06")
    );

    /// Set the persistent boot order
    #[tracing::instrument]
    pub async fn set_persistent_boot(
        &mut self,
        devices: Vec<Device>,
    ) -> Result<(), commands::Error> {
        assert_fw!(
            self.firmware(),
            "iLO 4 or iLO 3 or iLO 2 version >= 2.06",
            (Ilo4),
            (Ilo3),
            (Ilo2, "2.06")
        );
        let mut request = String::new();
        ribcl_command!(request, self.auth(), server_info, write, devices, {
            use std::fmt::Write;
            write!(request, "<set_persistent_boot>")?;
            for device in devices {
                write!(request, "<device value=\"{}\"/>", device.into_ribcl()?)?;
            }
            write!(request, "</set_persistent_boot/>")?;
        });
        let response = self.send(request.into_bytes()).await?;
        match ribcl_parse_response!(response) {
            Ok(_)
            | Err(crate::commands::Error::BuilderParse {
                source: crate::builder_parse::Error::NotFound { target: _ },
                ..
            }) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
