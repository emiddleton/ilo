use crate::{client, commands, types};
use std::{fs::File, io::Read, path::Path};

impl client::Node {
    get_method!(
        /// get firmware version
        rib_info.get_fw_version -> types::FwVersion
    );

    /// update firmware
    #[tracing::instrument]
    pub async fn update_rib_firmware(&mut self, path: &Path) -> Result<(), commands::Error> {
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;

        let mut request = String::new();
        ribcl_header!(request, self.auth(), rib_info, write, update_rib_firmware)?;
        use std::fmt::Write;
        write!(
            request,
            "<update_rib_firmware image_location=\"{}\" image_length=\"{}\"/>\r\n",
            &path.to_string_lossy(),
            &buffer.len()
        )?;
        let mut request = request.into_bytes();
        request.append(&mut buffer);
        Ok(())
    }
}
