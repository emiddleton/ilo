use crate::{
    client, commands,
    into_ribcl::IntoRibcl,
    ribcl_footer, ribcl_header, types,
    types::{BoolBuilder, Certificate, CertificateBuilder, SimpleBuilder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

//simple_builder_alias!(Login, String);
pub type Login = String;
pub type LoginBuilder = SimpleBuilder<Login>;

#[derive(Debug, Serialize, PartialEq)]
pub enum CertOwner {
    San,
    Subject,
}

pub type CertOwnerBuilder = SimpleBuilder<CertOwner>;
simple_builder_def!(
    CertOwner,
    {
        |value| {
            use CertOwner::*;
            match value.as_str() {
                "cert_owner_san" => Ok(Some(San)),
                "cert_owner_subject" => Ok(Some(Subject)),
                _ => Err(types::Error::InvalidString {
                    target: "CertOwner",
                    value,
                }),
            }
        }
    },
    {
        |value| {
            use CertOwner::*;
            match *value {
                San => "cert_owner_san",
                Subject => "cert_owner_subject",
            }
            .to_string()
        }
    }
);

#[skip_serializing_none]
#[derive(Debug, Default, BuilderParse, WriteRibcl, Serialize, PartialEq)]
pub struct TwofactorSettings {
    pub auth_twofactor_enable: Option<bool>,
    pub cert_revocation_check: Option<bool>,
    #[ribcl(empty)]
    pub cert_owner: Option<CertOwner>,
    pub import_ca_certificate: Option<Certificate>,
    pub import_user_certificate: Option<Certificate>,
    pub import_user_certificate_user_login: Option<Login>,
}

impl client::Node {
    #[tracing::instrument]
    pub async fn import_ssh_key(&mut self, content: String) -> Result<String, commands::Error> {
        // write
        unimplemented!()
    }

    get_method!(
        /// get two factor settings
        rib_info.get_twofactor_settings -> TwofactorSettings,
        "iL0 2 version >= 1.10",
        (Ilo2, "1.10")
    );

    mod_method!(
        /// set two factor settings
        rib_info.mod_twofactor_settings(TwofactorSettings),
        "iL0 2 version >= 1.10",
        (Ilo2, "1.10")
    );
}
