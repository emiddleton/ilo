extern crate anyhow;
extern crate ilo_console;

use crossbeam_channel::unbounded;
use ilo_console::{dvc, dvc::Decode, gui, transport};
use std::{fs::File, io::Read};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let (transport_tx, _transport_rx) = unbounded::<transport::Event>();
    let (gui_tx, gui_rx) = unbounded::<gui::Event>();
    let mut decoder = dvc::Decoder::new(gui_tx, transport_tx.clone());

    //let enc_header_pos = 0;

    let f = File::open("./decrypted_data.dat").unwrap();
    for cl in f.bytes() {
        let cl = cl.unwrap();
        println!("processing {}", cl);
        let dvc_mode = decoder.process_dvc(cl as u16);
        if !dvc_mode {
            panic!("finished encoded section");
        }
    }

    gui::handle(gui_rx, transport_tx)?;
    Ok(())
}
