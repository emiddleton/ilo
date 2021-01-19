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
use std::{fs, path::PathBuf};
use structopt::StructOpt;
use tokio::time::{interval, Duration};
use tracing::{event, Level};
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};
#[derive(Debug, StructOpt)]
#[structopt(name = "console", about = "kvm console for iLO 2 servers")]

struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// auth file
    #[structopt(short, long, parse(from_os_str), default_value = "auth.json")]
    auth: PathBuf,
}

//#[tokio::main]
fn main() -> Result<()> {
    let opt = Opt::from_args();

    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load config
    let auth_filename = opt.auth.as_path().display().to_string();
    let auth_json = fs::read_to_string(opt.auth)
        .with_context(|| format!("missing or invalid auth file {}", auth_filename))?;
    let auth: Auth = serde_json::from_str(auth_json.as_str())?;

    // setup communications
    let (transport_tx, transport_rx) = unbounded::<transport::Event>();
    let (gui_tx, gui_rx) = unbounded::<gui::Event>();

    // setup tokio runtime
    let rt = tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()?;

    rt.spawn(run_transport(
        auth,
        transport_rx,
        transport_tx.clone(),
        gui_tx,
    ));

    rt.spawn(keepalive(transport_tx.clone()));

    gui::handle(gui_rx, transport_tx)?;

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

async fn run_transport(
    mut auth: Auth,
    transport_rx: Receiver<transport::Event>,
    transport_tx: Sender<transport::Event>,
    gui_tx: Sender<gui::Event>,
) -> Result<()> {
    let params = auth.parameters().await?;
    let session = Session::try_from(params)?;

    // save session
    serde_json::to_writer_pretty(
        &fs::File::create("auth.json").context("couldn't open or create auth.json")?,
        &auth,
    )
    .context("can't write config to auth.json")?;

    let mut transport = Transport::new(session, transport_rx.clone(), gui_tx.clone());
    let mut decoder = Decoder::new(gui_tx, transport_tx.clone());

    transport.run(&mut decoder).await?;
    Ok(())
}
