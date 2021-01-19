use anyhow::Result;
use ilo_ribcl::{commands, parse_node_auth};
use ilo_ribcl_derive::ribcl_auth;
use structopt::StructOpt;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[ribcl_auth]
#[derive(Debug, StructOpt)]
#[structopt(name = "info", about = "use api to get information about endpoint")]
struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load auth
    let mut node = parse_node_auth!(opt);

    macro_rules! supported {
        ($section:ident.$command:ident) => {{
            match $section.$command().await {
                Ok(r) => {
                    println!(
                        "{}.{}() -> {}",
                        stringify!($section),
                        stringify!($command),
                        serde_json::to_string_pretty(&r)?
                    )
                }
                Err(commands::Error::NotSupported { requirements }) => {
                    println!(
                        "{}.{}() -> requires {:?}",
                        stringify!($section),
                        stringify!($command),
                        requirements
                    );
                    //event!(Level::WARN, ?requirements, " requires",)
                }
                Err(e) => {
                    println!(
                        "{}.{}() ->\n{}",
                        stringify!($section),
                        stringify!($command),
                        e
                    );
                    return Err(e).map_err(|e| e.into());
                }
            };
        }};
    }

    // security
    supported!(node.get_cert_subject_info);

    // logs
    supported!(node.get_ilo_event_log);
    supported!(node.get_server_event_log);

    // general
    supported!(node.get_host_data);
    supported!(node.get_global_settings);
    supported!(node.get_server_name);
    supported!(node.get_uid_status);
    supported!(node.get_all_languages);
    supported!(node.get_language);

    // network
    supported!(node.get_network_settings);

    // snmp
    supported!(node.get_snmp_im_settings);

    // authentication
    supported!(node.get_twofactor_settings);

    // ahs
    supported!(node.get_ahs_status);

    // virtual_media
    supported!(node.get_vm_status);

    // health
    supported!(node.get_embedded_health);

    // power
    supported!(node.get_host_power_status);
    supported!(node.get_host_power_reg_info);
    supported!(node.get_server_auto_pwr);
    supported!(node.get_power_readings);
    supported!(node.get_server_power_on_time);
    supported!(node.get_host_power_saver_status);
    supported!(node.get_power_cap);
    supported!(node.get_host_pwr_micro_ver);
    supported!(node.get_pwreg);

    // boot
    supported!(node.get_one_time_boot);
    supported!(node.get_persistent_boot);

    Ok(())
}
