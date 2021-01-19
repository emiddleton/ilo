use anyhow::Result;
use ilo_ribcl::parse_node_auth;
use ilo_ribcl_derive::ribcl_auth;
use structopt::StructOpt;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[ribcl_auth]
#[derive(Debug, StructOpt)]
#[structopt(
    name = "power",
    about = "use API to simulate pressing the power button"
)]
struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Cold boot node
    #[structopt(short, long)]
    cold_boot: bool,

    /// Warm boot node
    #[structopt(short, long)]
    warm_boot: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load auth
    let mut node = parse_node_auth!(opt);

    match (opt.cold_boot, opt.warm_boot) {
        (true, _) => node.cold_boot_server().await?,
        (_, true) => node.warm_boot_server().await?,
        _ => node.press_pwr_btn().await?,
    }
    Ok(())
}
