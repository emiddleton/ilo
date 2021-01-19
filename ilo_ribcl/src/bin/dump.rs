use anyhow::{Context, Result};
use ilo_ribcl::parse_node_auth;
use ilo_ribcl_derive::ribcl_auth;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

#[ribcl_auth]
#[derive(Debug, StructOpt)]
#[structopt(
    name = "dump",
    about = "contact server with supplied xml file and dump response"
)]
struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
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
    let auth = node.auth();
    let request = fs::read_to_string(opt.input).context("missing or invalid file auth.json")?;
    let re = Regex::new(r#"(?i)(LOGIN\s+USER_LOGIN=")\w+("\s+PASSWORD=")\w+(")"#)?;
    let result = format!("${{1}}{}${{2}}{}${{3}}", auth.username, auth.password);

    let request = re.replace(request.as_str(), result.as_str()).to_string();
    println!("{}", &request);
    let remove_comments_rxp = Regex::new("<!--.*-->")?;
    let request = remove_comments_rxp
        .replace_all(request.as_str(), "")
        .to_string();
    //let request = request.to_string().replace("\x0a", "");
    let remove_space_between_tags_rxp = Regex::new("\x0a\x20*").unwrap();
    let request = remove_space_between_tags_rxp
        .replace_all(request.as_str(), "")
        .to_string();
    let xml_document_rxp = Regex::new("^(<\\?xml version=\"1.0\"\\?>|)").unwrap();
    let request = xml_document_rxp.replace(request.as_str(), "<?xml version=\"1.0\"?>");
    //println!("{}", &request);
    //panic!("");

    let request = format!("{}\r\n", request);
    println!("{:?}", &request);
    let response = node.send(request.into_bytes()).await?;

    println!("{}", response);

    //let mut doc_iter = res.split(r#"<?xml version="1.0"?>"#,
    //doc_iter.next();
    println!("The End");
    Ok(())
}
