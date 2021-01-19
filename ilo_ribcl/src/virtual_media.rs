use crate::{
    client, commands,
    into_ribcl::IntoRibcl,
    ribcl_into::RibclInto,
    types::{BoolBuilder, Device, DeviceBuilder, StringBuilder, Url, UrlBuilder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[skip_serializing_none]
#[derive(Debug, WriteRibcl, Default, Serialize, PartialEq, BuilderParse)]
#[ribcl(attributes)]
pub struct VmStatus {
    pub vm_applet: Option<String>,
    pub device: Option<Device>,
    pub boot_option: Option<String>,
    pub write_protect: Option<bool>,
    pub image_inserted: Option<bool>,
    pub image_url: Option<Url>,
}

impl client::Node {
    get_method!(
        /// Get the status of virtual media devices
        rib_info.get_vm_status -> VmStatus
    );

    // TODO: rewrite to allow separate boot_option and write_protected arguments
    // mod_method!(
    //    /// Set the parameters of the virtual device specified virtual media.
    //    rib_info.set_vm_status("boot_option": "write_protect")
    // )
    mod_method!(
        /// Set the parameters of the virtual device specified virtual media.
        rib_info.set_vm_status(VmStatus)
    );

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
        // write
        unimplemented!()
    }

    mod_method!(
        /// Eject the virtual media device
        rib_info.eject_virtual_media("device": Device)
    );
}
