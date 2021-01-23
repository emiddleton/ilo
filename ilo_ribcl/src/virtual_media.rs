use crate::{
    client, commands,
    into_ribcl::IntoRibcl,
    ribcl_into::RibclInto,
    types::{BoolBuilder, Device, DeviceBuilder, StringBuilder, Url, UrlBuilder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::{convert::TryInto, fmt::Write};

#[skip_serializing_none]
#[derive(Debug, WriteRibcl, Default, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct VmStatus {
    pub vm_applet: Option<String>,   // CONNECTED | DISCONNECTED
    pub device: Option<Device>,      // FLOPPY || CDROM
    pub boot_option: Option<String>, // BOOT_ALWAYS | BOOT_ONCE | NO_BOOT
    pub write_protect: Option<bool>,
    pub image_inserted: Option<bool>,
    pub image_url: Option<Url>,
}

impl client::Node {
    /*
    get_method!(
        /// Get the status of virtual media devices
        rib_info.get_vm_status(Device) -> VmStatus
    );
    */
    /// Get the status of virtual media devices
    #[tracing::instrument]
    pub async fn get_vm_status(&mut self, device: Device) -> Result<VmStatus, commands::Error> {
        let mut request = String::new();
        ribcl_header!(request, self.auth(), rib_info, read)?;
        write!(
            request,
            "<get_vm_status device=\"{}\"/>",
            device.into_ribcl()?
        )?;
        ribcl_footer!(request, rib_info)?;
        let response = self.send(request.into_bytes()).await?;
        Ok(
            ribcl_parse_response!(response, "get_vm_status" -> VmStatus)?.map_err(|source| {
                commands::Error::BuilderParse {
                    target: "VmStatus",
                    source,
                }
            })?,
        )
    }

    // TODO: rewrite to allow separate boot_option and write_protected arguments
    // mod_method!(
    //    /// Set the parameters of the virtual device specified virtual media.
    //    rib_info.set_vm_status("boot_option": "write_protect")
    // )
    /*
    mod_method!(
        /// Set the parameters of the virtual device specified virtual media.
        rib_info.set_vm_status(VmStatus)
    );
    */
    /// Set the parameters of the virtual device specified virtual media.
    #[tracing::instrument]
    pub async fn set_vm_status(&mut self, vm_status: VmStatus) -> Result<(), commands::Error> {
        let mut request = String::new();
        ribcl_header!(request, self.auth(), rib_info, write)?;
        match vm_status.device {
            Some(device) => write!(
                request,
                "<set_vm_status device=\"{}\">",
                device.into_ribcl()?,
            )?,
            None => return Err(commands::Error::FieldMissing { target: "device" }),
        };
        ribcl_tag!(request, vm_status, boot_option: vm_boot_option);
        ribcl_tag!(request, vm_status, write_protect: vm_write_protect);
        write!(request, "</set_vm_status>")?;
        ribcl_footer!(request, rib_info)?;
        let response = self.send(request.into_bytes()).await?;
        mod_method!(@parse_response response)
    }

    /*
    mod_method!(
        /// Insert a virtual floppy or CDROM
        rib_info.insert_virtual_media("device": types::Device, "image_url": types::Url)
    )
    */
    /// Insert a virtual floppy or CDROM
    #[tracing::instrument]
    pub async fn insert_virtual_media(
        &mut self,
        device: Device,
        image_url: Url,
    ) -> Result<(), commands::Error> {
        let mut request = String::new();
        ribcl_header!(request, self.auth(), rib_info, write)?;
        write!(
            request,
            "<insert_virtual_media device=\"{}\" image_url=\"{}\"/>",
            device.into_ribcl()?,
            image_url.into_ribcl()?
        )?;
        ribcl_footer!(request, rib_info)?;
        let response = self.send(request.into_bytes()).await?;
        mod_method!(@parse_response response)
    }

    mod_method!(
        /// Eject the virtual media device
        rib_info.eject_virtual_media("device": Device)
    );
}
