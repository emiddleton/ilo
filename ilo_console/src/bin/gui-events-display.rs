use crossbeam_channel::unbounded;
use ilo_console::{gui, transport};
use serde_json::Deserializer;
use std::fs;

fn main() {
    let data = fs::read_to_string("gui-events.json").expect("failed to read events.json");
    //let data = fs::read("gui-events.bincode").expect("failed to read events.json");
    let (transport_tx, _transport_rx) = unbounded::<transport::Event>();
    let (gui_tx, gui_rx) = unbounded::<gui::Event>();
    let stream = Deserializer::from_str(&data).into_iter::<gui::Event>();
    //let evec: Vec<gui::Event> = bincode::deserialize(&data).unwrap();
    //let stream = evec.into_iter();
    for evnt in stream {
        let evnt = evnt.unwrap();
        //println!("{:?}", evnt);
        gui_tx.send(evnt).expect("failed to send event");
    }
    gui_tx.send(gui::Event::Exit).expect("failed to send event");
    let _ = gui::handle(gui_rx, transport_tx);
}
