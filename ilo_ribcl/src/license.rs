use crate::{client, commands};

impl client::Node {
    // get_all_licenses()

    /// Activate an iLO advanced license
    #[tracing::instrument]
    pub async fn activate_license(&mut self, license: String) -> Result<(), commands::Error> {
        let mut request = String::new();
        ribcl_command!(request, self.auth(), rib_info, write, license, {
            use std::fmt::Write;
            write!(request, "<activate key=\"{}\"/>", license)?;
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

    // deactivate_license()
}
