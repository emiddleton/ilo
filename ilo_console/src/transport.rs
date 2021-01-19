use std::io;
use thiserror::Error;

#[derive(Debug)]
pub enum Event {
    MouseMove,
    RequestRefresh,
    KeyPressed {
        keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
        repeat: bool,
    },
    KeyReleased {
        keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
        repeat: bool,
    },
    KeyTyped(String),
    SendKeepalive,
    UpdateEncryptionKey,
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("an io error occurred")]
    IoError(#[from] io::Error),
}
