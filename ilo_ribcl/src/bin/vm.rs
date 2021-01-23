use anyhow::{anyhow, Result};
use ilo_ribcl::{
    parse_node_auth,
    //power::PowerStatus,
    types::{Device, Url},
    virtual_media::VmStatus,
};
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
    // command on of mount, umount, status
    command: String,

    /// device type to mount i.e. floppy, cdrom
    device: Option<String>,

    /// url of image to mount.
    image_url: Option<Url>,
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
        "mount" => {
            let device = get_device(opt.device)?;
            node.insert_virtual_media(
                device,
                opt.image_url
                    .expect("image_url argument is required when mounting"),
            )
            .await?;
            let update = VmStatus {
                device: Some(device),
                boot_option: Some(String::from("CONNECT")),
                write_protect: Some(true),
                ..Default::default()
            };
            node.set_vm_status(update).await?;
            let status = node.get_vm_status(device).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        "umount" => node.eject_virtual_media(get_device(opt.device)?).await?,
        "boot" => {
            let device = get_device(opt.device)?;
            let update = VmStatus {
                device: Some(device),
                boot_option: Some(String::from("CONNECT")),
                write_protect: Some(true),
                ..Default::default()
            };
            node.set_vm_status(update).await?;
            node.set_one_time_boot(device).await?;
            let status = node.get_vm_status(device).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            //node.set_host_power(PowerStatus::Off).await?
        }
        "status" => {
            let device = get_device(opt.device)?;
            let status = node.get_vm_status(device).await?;
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        command => {
            return Err(anyhow!(
                "Invalid command: {}\nmust be one of mount umount",
                command
            ));
        }
    }
    Ok(())
}

fn get_device(device: Option<String>) -> Result<Device> {
    match device.as_deref() {
        Some("floppy") => Ok(Device::Floppy),
        Some("cdrom") => Ok(Device::Cdrom),
        Some(device_type) => Err(anyhow!(
            "Invalid device type: {}\nmust be one of floppy cdrom",
            device_type
        )),
        None => Err(anyhow!(
            "device type argument required: \nmust be one of floppy cdrom"
        )),
    }
}
