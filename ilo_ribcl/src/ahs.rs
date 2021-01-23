use crate::{
    client,
    into_ribcl::IntoRibcl,
    types::{BoolBuilder, SimpleBuilder},
};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

//simple_builder_alias!(AhsStatus, String);
pub type AhsStatus = String;
pub type AhsStatusBuilder = SimpleBuilder<AhsStatus>;

//simple_builder_alias!(AhsHardwareStatus, String);
pub type AhsHardwareStatus = String;
pub type AhsHardwareStatusBuilder = SimpleBuilder<AhsHardwareStatus>;

#[skip_serializing_none]
#[derive(BuilderParse, WriteRibcl, Debug, Default, Serialize, PartialEq)]
pub struct AhsStatusInfo {
    pub ahs_status: Option<AhsStatus>,
    pub ahs_hardware_status: Option<AhsHardwareStatus>,
    pub temp_ahs_disabled: Option<bool>,
}

impl client::Node {
    get_method!(
        /// Returns the Active Health System (AHS) logging status
        rib_info.get_ahs_status -> AhsStatusInfo, "iL0 4", (Ilo4)
    );

    mod_method!(
        /// Enable or disable Active Health System (AHS) logging
        rib_info.set_ahs_status(AhsStatusInfo),
        "iL0 4",
        (Ilo4)
    );

    /// Clear the Active Health System (AHS) logging data
    #[tracing::instrument]
    pub async fn ahs_clear_data(&mut self) -> Result<(), crate::commands::Error> {
        assert_fw!(self.firmware(), "iLO 4", (Ilo4));
        // write
        unimplemented!();
    }
}
