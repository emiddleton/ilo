use crate::{dvc, gui, ilo2::session::Session, rc4::Rc4, transport::Event};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::io;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};
use tracing::{event, instrument, Level};

#[derive(Debug)]
pub struct Transport {
    session: Session,
    transport_rx: Receiver<Event>,
    gui_tx: Sender<gui::Event>,
    stream: Option<TcpStream>,
    decryptor: Option<Rc4>,
    encrypter: Option<Rc4>,
    pub dvc_mode: bool,
    pub dvc_encryption: bool,
}

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("an io error occurred")]
    IoError(#[from] io::Error),
}
#[derive(PartialEq)]
enum Modifier {
    Shift,
    Ctrl,
    Alt,
    None,
}

impl Transport {
    #[instrument]
    pub fn new(
        session: Session,
        transport_rx: Receiver<Event>,
        gui_tx: Sender<gui::Event>,
    ) -> Transport {
        Transport {
            transport_rx,
            gui_tx,
            stream: None,
            encrypter: Some(Rc4::new(&session.encrypt_key)),
            decryptor: Some(Rc4::new(&session.decrypt_key)),
            session,
            dvc_mode: false,
            dvc_encryption: false,
        }
    }

    #[instrument(skip(self))]
    pub async fn key_typed(&mut self, key: String) -> Result<(), Error> {
        let data = Self::translate_key(&key);
        self.transmit(&data).await
    }

    #[instrument(skip(self))]
    pub async fn key_pressed(
        &mut self,
        keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
    ) -> Result<(), Error> {
        let data = Self::translate_special_key(keycode, keymod);
        self.transmit(&data).await
    }

    #[instrument(skip(self))]
    pub async fn key_released(
        &mut self,
        keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
    ) -> Result<(), Error> {
        let data = Self::translate_special_key_release(keycode, keymod);
        self.transmit(&data).await
    }

    #[instrument]
    fn translate_key(key: &str) -> Vec<u8> {
        match key {
            "`" => b"{".to_vec(),
            "{" => b"}".to_vec(),
            "}" => b"|".to_vec(),
            "=" => b"_".to_vec(),
            "~" => b")".to_vec(),
            ")" => b"(".to_vec(),
            "(" => b"*".to_vec(),
            "*" => b"\"".to_vec(),
            "\"" => b"@".to_vec(),
            "@" => b"[".to_vec(),
            "[" => b"]".to_vec(),
            "]" => b"\\".to_vec(),
            "+" => b":".to_vec(),
            ":" => b"\'".to_vec(),
            "'" => b"&".to_vec(),
            "&" => b"^".to_vec(),
            "¥" => b"\x00\xd4".to_vec(), // ô
            "\\" => b"\x00\xf2".to_vec(), // ò
            "_" => b"\x00\xf3".to_vec(),  // ó
            "|" => b"\x00\xf5".to_vec(),  // õ
            "^" => b"=".to_vec(),
            d => d.as_bytes().to_vec(),
        }
    }

    #[instrument]
    fn translate_special_key(
        keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
    ) -> Vec<u8> {
        use sdl2::keyboard::Mod;
        let mut i = 1;

        let modifier = if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
            Modifier::Shift
        } else if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
            Modifier::Ctrl
        } else if keymod.contains(Mod::LALTMOD) || keymod.contains(Mod::RALTMOD) {
            Modifier::Alt
        } else {
            Modifier::None
        };

        macro_rules! mods {
            ($none:literal, $shift:literal, $ctrl:literal, $alt:literal) => {{
                i = 0;
                match modifier {
                    Modifier::None => $none,
                    Modifier::Shift => $shift,
                    Modifier::Ctrl => $ctrl,
                    Modifier::Alt => $alt,
                }
            }};
        }

        use sdl2::keyboard::Keycode::*;
        let mut str = match keycode {
            Escape => "\x1b",
            Tab => "\t",
            Delete => {
                if modifier == Modifier::Ctrl && keymod.contains(Mod::LALTMOD)
                    || keymod.contains(Mod::RALTMOD)
                {
                    // TODO: implement reboot
                    event!(Level::WARN, "send CTRL-ALT-DEL");
                }
                "\x7f"
            }
            Home => "\x1b[H",                                     // 36
            End => "\x1b[F",                                      // 35
            PageUp => "\x1b[I",                                   // 33
            PageDown => "\x1b[G",                                 // 34
            Insert => "\x1b[L",                                   // 155
            Up => "\x1b[A",                                       // 38
            Down => "\x1b[B",                                     // 40
            Left => "\x1b[D",                                     // 37
            Right => "\x1b[C",                                    // 39
            F1 => mods!("\x1b[M", "\x1b[Y", "\x1b[k", "\x1b[w"),  // 112
            F2 => mods!("\x1b[N", "\x1b[Z", "\x1b[l", "\x1b[x"),  // 113
            F3 => mods!("\x1b[O", "\x1b[a", "\x1b[m", "\x1b[y"),  // 114
            F4 => mods!("\x1b[P", "\x1b[b", "\x1b[n", "\x1b[z"),  // 115
            F5 => mods!("\x1b[Q", "\x1b[c", "\x1b[o", "\x1b[@"),  // 116
            F6 => mods!("\x1b[R", "\x1b[d", "\x1b[p", "\x1b[["),  // 117
            F7 => mods!("\x1b[S", "\x1b[e", "\x1b[q", "\x1b[\\"), // 118
            F8 => mods!("\x1b[T", "\x1b[f", "\x1b[r", "\x1b[]"),  // 119
            F9 => mods!("\x1b[U", "\x1b[g", "\x1b[s", "\x1b[^"),  // 120
            F10 => mods!("\x1b[V", "\x1b[h", "\x1b[t", "\x1b[_"), // 121
            F11 => mods!("\x1b[W", "\x1b[i", "\x1b[u", "\x1b[`"), // 122
            F12 => mods!("\x1b[X", "\x1b[j", "\x1b[v", "\x1b['"), // 123
            Return | Return2 => mods!("\r", "\x1b[3\r", "\n", "\x1b[1\r"),
            Backspace => mods!("\x08", "\x1b[3\x08", "\x7f", "\x1b[1\x08"),
            _ => "",
        }
        .to_string();
        if !str.is_empty() && i == 1 {
            str = match modifier {
                Modifier::Shift => format!("\x1b[3{}", str),
                Modifier::Ctrl => format!("\x1b[2{}", str),
                Modifier::Alt => format!("\x1b[1{}", str),
                _ => str,
            }
        }
        str.into_bytes()
    }

    #[instrument]
    fn translate_special_key_release(
        _keycode: sdl2::keyboard::Keycode,
        keymod: sdl2::keyboard::Mod,
    ) -> Vec<u8> {
        use sdl2::keyboard::Mod;

        let mut i: u8 = 0b0000_0000;
        if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
            i |= 0b0000_0001
        }
        if keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD) {
            i |= 0b0000_0010
        }
        if keymod.contains(Mod::LALTMOD) || keymod.contains(Mod::RALTMOD) {
            i |= 0b0000_0100
        }
        if i > 0b0111_1111 {
            vec![0x00, i]
        } else {
            vec![0x00]
        }
    }

    #[instrument(skip(self))]
    pub async fn refresh_screen(&mut self) -> Result<(), Error> {
        self.transmit("\x1b[~".as_bytes()).await
    }

    #[instrument(skip(self))]
    pub async fn send_auto_alive_msg(&mut self) -> Result<(), Error> {
        self.transmit("\x00\x1b[&".as_bytes()).await
    }

    #[instrument(skip(self))]
    pub async fn send_keep_alive_msg(&mut self) -> Result<(), Error> {
        self.transmit("\x1b[(".as_bytes()).await
    }

    #[instrument(skip(self))]
    pub async fn send_ctrl_alt_del(&mut self) -> Result<(), Error> {
        self.transmit("\x1b[2\x1b[\x7f".as_bytes()).await
    }

    // synchronized
    #[instrument(skip(self))]
    pub async fn connect(&mut self) -> Result<(), Error> {
        match self.stream {
            None => {
                let mut stream =
                    TcpStream::connect(format!("{}:{}", self.session.host, self.session.port))
                        .await?;
                stream.set_linger(None).unwrap_or_else(|_e| {
                    event!(Level::WARN, "Failed to set linger on socket");
                });
                stream.set_nodelay(true)?;

                event!(Level::DEBUG, "Online");

                // append bytes to login if encrypted
                let mut login: Vec<u8> = self.session.login.clone().as_bytes().to_vec();
                if self.session.encryption_enabled {
                    // append to first command 'ÿÀ    '
                    let mut prefix = vec![0xFF, 0xC0, 0x20, 0x20, 0x20, 0x20];
                    prefix.append(&mut login);
                    login = prefix
                };

                self.transmit_encrypt_command(&mut stream, &login).await?;

                self.stream = Some(stream);
            }
            _ => event!(Level::INFO, "already connected"),
        }

        Ok(())
    }

    #[instrument(skip(self, stream))]
    pub async fn transmit_encrypt_command<A>(
        &mut self,
        stream: &mut A,
        data: &[u8],
    ) -> Result<(), Error>
    where
        A: AsyncWriteExt + Unpin,
    {
        let mut out: Vec<u8> = vec![0; data.len()];
        if let Some(encrypter) = &mut self.encrypter {
            event!(Level::DEBUG, self.session.key_index);

            out[0] = data[0];
            out[1] = data[1];
            out[2] = ((self.session.key_index & 0xFF000000) >> 24) as u8;
            out[3] = ((self.session.key_index & 0xFF0000) >> 16) as u8;
            out[4] = ((self.session.key_index & 0xFF00) >> 8) as u8;
            out[5] = (self.session.key_index & 0xFF) as u8;

            event!(Level::TRACE, transmitting_buffer = ?hex::encode(&out));
            encrypter.process_bytes(&data[6..], &mut out[6..]);
        }
        event!(
            Level::TRACE,
            data=?hex::encode(&data),
            data_len=data.len(),
            out = ?hex::encode(&out),
            out_len=out.len()
        );
        stream.write_all(&out[..]).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn transmit(&mut self, data: &[u8]) -> Result<(), Error> {
        if let Some(stream) = &mut self.stream {
            let mut out: Vec<u8> = vec![0; data.len()];
            if let Some(encrypter) = &mut self.encrypter {
                encrypter.process_bytes(data, &mut out[..]);
                event!(
                    Level::TRACE,
                    data=?hex::encode(&data),
                    data_len=data.len(),
                    out=?hex::encode(&out),
                    out_len=&out.len()
                );
            }
            stream.write_all(&out[..]).await?;
        }
        Ok(())
    }

    #[instrument(skip(self, decoder))]
    pub async fn run<D>(&mut self, decoder: &mut D) -> Result<(), Error>
    where
        D: dvc::Decode,
    {
        let mut buffer: [u8; 1024] = [0; 1024];

        let mut enc_header_pos: u32 = 0;

        self.connect().await?;

        'main: loop {
            loop {
                event!(Level::DEBUG, "processing transport_tx events");
                use Event::*;
                match self.transport_rx.try_recv() {
                    Ok(ev) => {
                        event!(Level::DEBUG, ?ev);
                        match ev {
                            RequestRefresh => {
                                println!("receive Request Refresh");
                                self.refresh_screen().await?;
                            }
                            MouseMove => {}
                            KeyPressed {
                                keycode, keymod, ..
                            } => self.key_pressed(keycode, keymod).await?,
                            KeyReleased {
                                keycode, keymod, ..
                            } => self.key_released(keycode, keymod).await?,
                            KeyTyped(key) => self.key_typed(key).await?,
                            SendKeepalive => self.send_keep_alive_msg().await?,
                            UpdateEncryptionKey => {
                                event!(Level::INFO, "updating encryption key");
                                if let Some(encrypter) = &mut self.encrypter {
                                    encrypter.update_key();
                                }
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        event!(Level::DEBUG, "transport_tx is empty");
                        break;
                    }
                    Err(TryRecvError::Disconnected) => {
                        event!(Level::ERROR, "disconnected from server");
                        break 'main;
                    }
                }
            }

            let n: usize = if let Some(stream) = &mut self.stream {
                event!(Level::DEBUG, "read new data");
                if let Ok(result) =
                    timeout(Duration::from_millis(10), stream.read(&mut buffer)).await
                {
                    result?
                } else {
                    0
                }
            } else {
                event!(Level::ERROR, "no tcp stream");
                break;
            };

            if n > 0 {
                event!(Level::DEBUG, "received new data");
            } else {
                event!(Level::DEBUG, "received no new data");
            }

            let buffer_iter = buffer[..n].iter();
            for next_byte in buffer_iter {
                event!(Level::TRACE, next_byte, self.dvc_mode);
                if self.dvc_mode {
                    let d: u16 = if let Some(decryptor) = &mut self.decryptor {
                        decryptor.process_byte(*next_byte)
                    } else {
                        *next_byte
                    } as u16;
                    event!(Level::TRACE, encrypted_byte = next_byte, decrypted_byte = ?d);
                    self.dvc_mode = decoder.process_dvc(d);

                    if !self.dvc_mode {
                        event!(Level::WARN, "DVC mode turned off");
                        self.gui_tx
                            .send(gui::Event::ShowText {
                                text: "DVC mode turned off".to_string(),
                            })
                            .unwrap();
                    }
                } else {
                    // find start of encryption sequence
                    enc_header_pos = match next_byte {
                        0x1b => 1,
                        b'[' if enc_header_pos == 1 => 2,
                        b'R' | b'r' if enc_header_pos == 2 => {
                            self.dvc_mode = true;
                            if b'R' == *next_byte {
                                self.dvc_encryption = true;
                                event!(Level::INFO, "DVC Mode (RC4-128 bit)");
                            } else {
                                self.dvc_encryption = false;
                                event!(Level::INFO, "DVC Mode (no encryption)");
                            }
                            break;
                        }
                        _ => 0,
                    }
                }
            }
        }
        Ok(())
    }
}
