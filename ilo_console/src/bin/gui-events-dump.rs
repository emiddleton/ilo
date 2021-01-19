extern crate anyhow;
extern crate ilo_console;
extern crate regex;
extern crate reqwest;

use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ilo_console::{
    dvc::Decoder,
    gui,
    ilo2::{auth::Auth, session::Session, transport::Transport},
    transport,
};
use std::{
    fs,
    io::{stdout, Write},
};
use tokio::time::{interval, Duration};
use tracing::{event, Level};
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

//#[tokio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load config
    let auth_json = fs::read_to_string("auth.json").expect("missing or invalid file auth.json");
    let auth: Auth = serde_json::from_str(auth_json.as_str())?;

    // setup communications
    let (transport_tx, transport_rx) = unbounded::<transport::Event>();
    let (proxy_gui_tx, proxy_gui_rx) = unbounded::<gui::Event>();
    let (gui_tx, gui_rx) = unbounded::<gui::Event>();

    // setup tokio runtime
    let mut rt = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()?;

    rt.spawn(run_transport(
        auth,
        transport_rx,
        transport_tx.clone(),
        proxy_gui_tx,
    ));

    let writer: Box<dyn Write + Send> = if true {
        Box::new(
            fs::File::create("gui-events.json")
                .context("couldn't open or create events.json")
                .unwrap(),
        )
    } else {
        Box::new(stdout())
    };
    rt.spawn(keepalive(transport_tx.clone()));
    rt.block_on(gui_events_dump(writer, proxy_gui_rx, gui_tx))?;

    if false {
        gui::handle(gui_rx, transport_tx)?;
    }

    rt.shutdown_timeout(Duration::from_millis(100));
    event!(Level::DEBUG, "shutting down");
    Ok(())
}

async fn keepalive(transport_tx: Sender<transport::Event>) -> Result<()> {
    let mut keepalive_interval = interval(Duration::from_millis(15000));
    loop {
        keepalive_interval.tick().await;
        transport_tx.send(transport::Event::SendKeepalive)?;
    }
}

async fn gui_events_dump(
    mut writer: Box<dyn Write + Send>,
    rx: Receiver<gui::Event>,
    tx: Sender<gui::Event>,
) -> Result<()> {
    loop {
        if let Ok(ev) = rx.recv() {
            serde_json::to_writer(&mut writer, &ev)
                .context("can't write event to events.json")
                .unwrap();
            writer.write_all(b"\n").unwrap();
            /*
            let bev = bincode::serialize(&ev).unwrap();
            writer.write(&bev).unwrap();
            */
            tx.send(ev).unwrap();
        }
    }
}

async fn run_transport(
    mut auth: Auth,
    transport_rx: Receiver<transport::Event>,
    transport_tx: Sender<transport::Event>,
    gui_tx: Sender<gui::Event>,
) -> Result<()> {
    let params = auth.parameters().await?;
    let session = Session::try_from(params)?;

    let mut transport = Transport::new(session, transport_rx.clone(), gui_tx.clone());
    let mut decoder = Decoder::new(gui_tx, transport_tx.clone());

    transport.run(&mut decoder).await?;
    Ok(())
}
