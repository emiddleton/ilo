use crate::{
    builder_parse::BuilderParse as TraitBuilderParse,
    client, commands,
    types::{NaiveDateTimeBuilder, StringBuilder, U32Builder},
    xml::XmlCursor,
};
use chrono::naive::NaiveDateTime;
use ilo_ribcl_derive::BuilderParse;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

#[skip_serializing_none]
#[derive(Debug, Serialize, PartialEq, BuilderParse)]
pub struct ProLiantKey {
    #[ribcl(map = "kver")]
    key_version: u32,
    #[ribcl(map = "ltype")]
    license_type: String,
    #[ribcl(map = "lname")]
    license_name: Option<String>,
    #[ribcl(map = "idate")]
    install_date: NaiveDateTime,
    key: String,
    #[ribcl(map = "sn")]
    serial_number: String,
    sbsn: String,
}

impl client::Node {
    // Returns information about the currently installed license key
    #[tracing::instrument]
    pub async fn get_license(&mut self) -> Result<ProLiantKey, commands::Error> {
        let response = self.get_xmldata("CpqKey").await?;
        let (mut xml_cursor, root) = XmlCursor::new(&response)?;
        let builder: ProLiantKeyBuilder =
            xml_cursor.builder_parse(root, None).map_err(|source| {
                commands::Error::BuilderParse {
                    target: "ProLiantKeyBuilder",
                    source,
                }
            })?;
        Ok(builder
            .try_into()
            .map_err(|source| commands::Error::BuilderParse {
                target: "ProLiantKeyBuilder",
                source,
            })?)
    }

    /// Activate an iLO advanced license
    #[tracing::instrument]
    pub async fn activate_license(&mut self, license: String) -> Result<(), commands::Error> {
        let mut request = String::new();
        ribcl_command!(request, self.auth(), rib_info, write, license, {
            use std::fmt::Write;
            write!(request, "<activate key=\"{}\"/>", license)?;
        });
        let response = self.send_ribcl(request.into_bytes()).await?;
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
