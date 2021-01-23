use crate::{
    client, commands,
    into_ribcl::IntoRibcl,
    types::{BoolBuilder, Certificate, StringBuilder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[skip_serializing_none]
#[derive(WriteRibcl, Debug, Default, Serialize, PartialEq, BuilderParse)]
pub struct CsrCertSettings {
    #[ribcl(map = "csr_use_cert_2048pkey")]
    pub csr_use_cert_2048_pkey: Option<bool>,
    pub csr_use_cert_custom_subject: Option<bool>,
    pub csr_use_cert_fqdn: Option<bool>,
    pub csr_subject_common_name: Option<String>,
    pub csr_subject_country: Option<String>,
    pub csr_subject_state: Option<String>,
    pub csr_subject_location: Option<String>,
    pub csr_subject_org_name: Option<String>,
    pub csr_subject_orgunit_name: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum ComputerLock {
    Windows,
    Disabled,
    Custom { key: String },
}

impl client::Node {
    /// Configures whether to use the fqdn or the short hostname for certificate requests
    #[tracing::instrument]
    pub async fn cert_fqdn(&mut self, value: bool) -> Result<String, commands::Error> {
        // write
        assert_fw!(self.firmware(), "iLO 2", (Ilo2));
        unimplemented!()
    }

    get_method!(
        /// Returns the certificate subject info
        rib_info.get_cert_subject_info -> "csr_cert_settings" : CsrCertSettings,
        "iL0 2 version >= 2.06",
        (Ilo2, "2.06")
    );

    mod_method!(
        /// Updates the certificate subject info
        rib_info.csr_cert_settings(CsrCertSettings),
        "iLO 2 version >= 2.06",
        (Ilo2, "2.06")
    );
    /*
    mod_method!(
        /// set certificate subject info
        ///  Get a certificate signing request from the iLO
        rib_info.certificate_signing_request : "csr_cert_settings" (types::CsrCertSettings),
        "iLO 2 version >= 2.06",
        (Ilo2, "2.06")
    );

    mod_method!(
        rib_info.certificate_signing_request -> types::CertificateSigningRequest
    );
    */
    /// Returns a certificate signing request from the iLO
    #[tracing::instrument]
    pub async fn certificate_signing_request(&mut self) -> Result<String, commands::Error> {
        // write
        unimplemented!()
    }

    /// Import a signed SSL certificate
    #[tracing::instrument]
    pub async fn import_certificate(
        &mut self,
        content: Certificate,
    ) -> Result<String, commands::Error> {
        // write
        assert_fw!(self.firmware(), "iLO 2 version >= 1.70", (Ilo2, "1.70"));
        unimplemented!()
    }

    /// Updates the computer lock setting
    #[tracing::instrument]
    pub async fn computer_lock_config(
        &mut self,
        compute_lock: ComputerLock,
    ) -> Result<(), commands::Error> {
        assert_fw!(
            self.firmware(),
            "iLO 4 or iLO 3 or iLO 2 version >= 1.30",
            (Ilo4),
            (Ilo3),
            (Ilo2, "1.30")
        );
        let mut request = String::new();
        ribcl_command!(
            request,
            self.auth(),
            rib_info,
            write,
            computer_lock_config,
            {
                use std::fmt::Write;
                use ComputerLock::*;
                match compute_lock {
                    Windows => write!(request, r#"<computer_lock value="windows"/>"#)?,
                    Disabled => write!(request, r#"<computer_lock value="disabled"/>"#)?,
                    Custom { key } => {
                        write!(request, r#"<computer_lock value="disabled"/>"#)?;
                        write!(request, "<computer_lock_key value=\"{}\"/>", key)?;
                    }
                }
            }
        );
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
