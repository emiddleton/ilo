use anyhow::{anyhow, Result};
use ilo_ribcl::{parse_node_auth, power::PowerStatus};
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
    /// Is one of on or off
    command: String,

    /// Force cold boot or Shutdown.
    #[structopt(short, long)]
    force: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load auth
    let mut node = parse_node_auth!(opt);

    match opt.command.as_str() {
        "on" => {
            if opt.force {
                node.cold_boot_server().await?;
            } else {
                node.set_host_power(PowerStatus::On).await?;
            }
        }
        "off" => {
            if opt.force {
                if let PowerStatus::On = node.get_host_power_status().await? {
                    node.hold_pwr_btn().await?;
                } else {
                    println!("the server is already powered off");
                }
            } else {
                node.set_host_power(PowerStatus::Off).await?;
            }
        }
        command => {
            return Err(anyhow!(
                "Invalid command: {}\nmust be one of on off",
                command
            ));
        }
    }
    Ok(())
}
