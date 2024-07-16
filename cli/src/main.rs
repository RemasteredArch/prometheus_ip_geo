#![allow(dead_code)]
use clap::Parser;
use ip_geo::{parse_ipv4_file, parse_ipv6_file};
use serde::Deserialize;
use std::{
    fmt::Display,
    fs,
    net::{Ipv4Addr, Ipv6Addr},
    path::Path,
};

fn main() {
    let arguments = get_config(Arguments::parse());

    let mut ipv4_map = parse_ipv4_file(arguments.ipv4_path.unwrap(), arguments.ipv4_len.unwrap());

    for ipv4_addr in ipv4_map {
        println!("{:?}", ipv4_addr);
    }

    let mut ipv6_map = parse_ipv6_file(arguments.ipv6_path.unwrap(), arguments.ipv6_len.unwrap());

    for ipv6_addr in ipv6_map {
        println!(
            "{:39}\t{:39}\t{}",
            ipv6_addr.start(),
            ipv6_addr.end(),
            ipv6_addr.value().long_name
        );
    }

    let input_addr = arguments.ipv4_addr.unwrap();
    println!("{}", input_addr);

    //println!("{}", ipv4_map.search(input_addr).unwrap().long_name);
}

#[derive(Parser, Deserialize)]
#[command(about, version, long_about = None)]
struct Arguments {
    #[arg(short = 'f', long = "config-path")]
    #[serde(skip, default)]
    config_path: Option<Box<Path>>,

    #[arg(short = '4', long = "IPv4-addr")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv4_addr: Option<Ipv4Addr>,

    #[arg(long = "IPv4-path")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv4_path: Option<Box<Path>>,

    #[arg(long = "IPv4-length")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv4_len: Option<usize>,

    #[arg(long = "IPv4-comment")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv4_comment: Option<char>,

    #[arg(short = '6', long = "IPv6-addr")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv6_addr: Option<Ipv6Addr>,

    #[arg(long = "IPv6-path")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv6_path: Option<Box<Path>>,

    #[arg(long = "IPv6-length")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv6_len: Option<usize>,

    #[arg(long = "IPv6-comment")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    ipv6_comment: Option<char>,

    #[arg(short = 's', long = "server")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    server: Option<bool>,

    #[arg(short = 'p', long = "port")]
    #[serde(skip_serializing_if = "Option::is_none", default)]
    port: Option<u16>,
}

impl Display for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Config:")?;
        writeln!(f, " * Config: {:?}", self.config_path)?;
        writeln!(f, " * IPv4 DB: {:?}", self.ipv4_path)?;
        writeln!(f, " * IPv6 DB: {:?}", self.ipv6_path)?;
        writeln!(f, " * Start as server: {:?}", self.server)?;
        writeln!(f, " * Server port: {:?}", self.port)
    }
}

fn get_config(arguments: Arguments) -> Arguments {
    let from_config = get_config_file_arguments(&arguments).and_then(|v| v.ok());

    // does this need to be read from config file?
    let config = arguments
        .config_path
        .or_else(|| from_config.as_ref().and_then(|v| v.config_path.clone()))
        .unwrap_or_else(get_default_config_path);

    let ipv4_path = arguments
        .ipv4_path
        .unwrap_or_else(|| Path::new("/usr/share/tor/geoip").into());

    let ipv4_len = arguments
        .ipv4_len
        .or_else(|| from_config.as_ref().and_then(|v| v.ipv4_len))
        .unwrap_or(200_000);

    let ipv4_comment = arguments
        .ipv4_comment
        .or_else(|| from_config.as_ref().and_then(|v| v.ipv4_comment))
        .unwrap_or('#');

    let ipv6_path = arguments
        .ipv6_path
        .or_else(|| from_config.as_ref().and_then(|v| v.ipv6_path.clone()))
        .unwrap_or_else(|| Path::new("/usr/share/tor/geoip6").into());

    let ipv6_len = arguments
        .ipv6_len
        .or_else(|| from_config.as_ref().and_then(|v| v.ipv6_len))
        .unwrap_or(60_000);

    let ipv6_comment = arguments
        .ipv6_comment
        .or_else(|| from_config.as_ref().and_then(|v| v.ipv6_comment))
        .unwrap_or('#');

    let server = arguments
        .server
        .or_else(|| from_config.as_ref().and_then(|v| v.server))
        .unwrap_or_default();

    let port = arguments
        .port
        .or_else(|| from_config.as_ref().and_then(|v| v.port))
        .unwrap_or(26_000);

    Arguments {
        config_path: Some(config),
        ipv4_addr: arguments.ipv4_addr,
        ipv4_path: Some(ipv4_path),
        ipv4_len: Some(ipv4_len),
        ipv4_comment: Some(ipv4_comment),
        ipv6_addr: arguments.ipv6_addr,
        ipv6_path: Some(ipv6_path),
        ipv6_len: Some(ipv6_len),
        ipv6_comment: Some(ipv6_comment),
        server: Some(server),
        port: Some(port),
    }
}

fn get_config_file_arguments(arguments: &Arguments) -> Option<Result<Arguments, toml::de::Error>> {
    let config_path = arguments
        .config_path
        .clone()
        .unwrap_or_else(get_default_config_path);

    let contents = fs::read_to_string(&config_path).ok()?;
    Some(toml::from_str(&contents))
}

fn get_default_config_path() -> Box<Path> {
    dirs::config_dir()
        .unwrap()
        .join(env!("CARGO_PKG_NAME"))
        .with_extension("toml")
        .into_boxed_path()
}
