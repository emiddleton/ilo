use crate::{client, into_ribcl::IntoRibcl, types::SimpleBuilder};
use ilo_ribcl_derive::{BuilderParse, WriteRibcl};
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::convert::TryInto;

//simple_builder_alias!(KeyCharacter, String);
pub type KeyCharacter = String;
pub type KeyCharacterBuilder = SimpleBuilder<KeyCharacter>;

#[skip_serializing_none]
#[derive(BuilderParse, WriteRibcl, Debug, Default, Serialize, PartialEq)]
pub struct HotkeyConfig {
    pub ctrl_t: Option<KeyCharacter>,
    pub ctrl_u: Option<KeyCharacter>,
    pub ctrl_v: Option<KeyCharacter>,
    pub ctrl_w: Option<KeyCharacter>,
    pub ctrl_x: Option<KeyCharacter>,
    pub ctrl_y: Option<KeyCharacter>,
}

//trait KeyboardMouse {
impl client::Node {
    get_method!(
        /// Returns the hotkey options
        rib_info.get_hotkey_config -> HotkeyConfig,
        "iLO 2",
        (Ilo2)
    );

    mod_method!(
        /// Updates the hotkey options
        rib_info.hotkey_config(HotkeyConfig),
        "iLO 2",
        (Ilo2)
    );
}
