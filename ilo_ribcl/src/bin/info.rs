use anyhow::Result;
use ilo_ribcl::{commands, parse_node_auth, types::Device};
use ilo_ribcl_derive::ribcl_auth;
use structopt::StructOpt;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[ribcl_auth]
#[derive(Debug, StructOpt)]
#[structopt(name = "info", about = "use api to get information about endpoint")]
struct Opt {
    /// Command to run can all to run all commands or blank to show available commands
    #[structopt(default_value = "")]
    command: String,

    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Fail on error
    #[structopt(short, long)]
    fail: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opts = Opt::from_args();

    // setup tracing
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?; //.add_directive(LevelFilter::INFO.into());
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // load auth
    let mut node = parse_node_auth!(opts);

    macro_rules! supported {
        ($section:ident.$command:ident, $opts:ident, $description:literal) => {{
            supported!($section.$command(), $opts, $description);
        }};
        ($section:ident.$command:ident($($args:tt)*), $opts:ident, $description:literal) => {{
            match $opts.command.as_str() {
                stringify!($command) | "all" => {
                    match $section.$command($($args)*).await {
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
                        }
                        Err(e) => {
                            println!(
                                "{}.{}() ->\n{}",
                                stringify!($section),
                                stringify!($command),
                                e
                            );
                            if $opts.fail {
                                return Err(e).map_err(|e| e.into());
                            }
                        }
                    };
                }
                "" => {
                    println!("  {} {}", supported!(@command $command($($args)*)),$description);
                }
                _ => {}
            }
        }};
        (@command $command:tt()) => { stringify!($command) };
        (@command $command:tt($($args:tt)+)) => { stringify!($command($($args)+)) };
    }

    if opts.command.is_empty() {
        println!("Available commands:");
    }

    // license
    supported!(node.get_license, opts, "                - License info");

    // security
    supported!(
        node.get_cert_subject_info,
        opts,
        "      - Certificate subject info"
    );

    // logs
    supported!(node.get_ilo_event_log, opts, "          - iLO event log");
    supported!(
        node.get_server_event_log,
        opts,
        "       - IML log of the server"
    );

    // general
    supported!(node.get_host_data, opts, "              - host data");
    supported!(node.get_global_settings, opts, "        - global settings");
    supported!(node.get_server_name, opts, "            - server name");
    supported!(
        node.get_uid_status,
        opts,
        "             - server UID status"
    );
    supported!(
        node.get_all_languages,
        opts,
        "          - all supported languages"
    );
    supported!(
        node.get_language,
        opts,
        "               - selected language"
    );

    // network
    supported!(
        node.get_network_settings,
        opts,
        "       - iLO network settings"
    );

    // snmp
    supported!(node.get_snmp_im_settings, opts, "       - SNMP IM settings");

    // authentication
    supported!(
        node.get_twofactor_settings,
        opts,
        "     - iLO Two-Factor Authentication Settings"
    );

    // ahs
    supported!(
        node.get_ahs_status,
        opts,
        "             - active health system logging status"
    );

    // virtual_media
    let floppy = Device::Floppy;
    supported!(
        node.get_vm_status(floppy),
        opts,
        "     - virtual floppy device status"
    );
    let cdrom = Device::Cdrom;
    supported!(
        node.get_vm_status(cdrom),
        opts,
        "      - virtual cdrom device status"
    );

    // health
    supported!(
        node.get_embedded_health,
        opts,
        "        - health information of the server"
    );

    // power
    supported!(
        node.get_host_power_status,
        opts,
        "      - power state of the server"
    );
    supported!(
        node.get_host_power_reg_info,
        opts,
        "    - iLO power regulator info"
    );
    supported!(
        node.get_server_auto_pwr,
        opts,
        "        - Servers automatic power on and delay settings"
    );
    supported!(
        node.get_power_readings,
        opts,
        "         - Servers power reading from power supply"
    );
    supported!(
        node.get_server_power_on_time,
        opts,
        "   - Servers power on time"
    );
    supported!(
        node.get_host_power_saver_status,
        opts,
        "- Servers processor power regulator"
    );
    supported!(node.get_power_cap, opts, "              - Server power cap");
    supported!(
        node.get_host_pwr_micro_ver,
        opts,
        "     - Host power micro version"
    );
    // supported!(node.get_pwreg_capabilities, opts, "     - iLO power regulator information");
    supported!(
        node.get_pwreg,
        opts,
        "                  - Power alert threshold settings"
    );

    // boot
    supported!(
        node.get_one_time_boot,
        opts,
        "          - One time boot device"
    );
    supported!(
        node.get_persistent_boot,
        opts,
        "        - Persistent boot devices"
    );

    Ok(())
}
