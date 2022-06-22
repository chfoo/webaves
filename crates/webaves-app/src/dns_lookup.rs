use std::net::SocketAddr;

use clap::{Arg, ArgAction, ArgMatches, Command};
use webaves::dns::{Resolver, ResolverBuilder};

use crate::{
    argutil::DoHAddress,
    common::{BIND_ADDRESS_HELP, BIND_ADDRESS_HELP_LONG},
};

const ABOUT: &str = "Lookup DNS records";
const ABOUT_LONG: &str = "Lookup DNS IP addresses and records.

This command can be used to diagnose DNS resolver issues. It does not use \
the operating system's resolver but contacts servers on the internet directly.";
const ADDRESS_ABOUT: &str = "Lookup IP addresses for a hostname";
const RECORD_ABOUT: &str = "Lookup records for a hostname";
const HOSTNAME_HELP: &str = "Target hostname to query";
const DNS_RECORD_TYPE_HELP: &str = "DNS record type as string or integer";
const DOH_SERVER_HELP: &str = "Address and hostname of DNS-over-HTTPS server";
const DOH_SERVER_HELP_LONG: &str = "Address and hostname of DNS-over-HTTPS server.

Example: \"10.0.0.0:443/dns.example.com\" specifies IP address 10.0.0.0, \
port number 443, and a hostname of dns.example.com.";

pub fn create_command() -> Command<'static> {
    let address_command = Command::new("address")
        .about(ADDRESS_ABOUT)
        .arg(Arg::new("hostname").required(true).help(HOSTNAME_HELP));

    let record_command = Command::new("record")
        .about(RECORD_ABOUT)
        .arg(Arg::new("type").required(true).help(DNS_RECORD_TYPE_HELP))
        .arg(Arg::new("hostname").required(true).help(HOSTNAME_HELP));

    Command::new("dns-lookup")
        .about(ABOUT)
        .long_about(ABOUT_LONG)
        .subcommand_required(true)
        .arg(
            Arg::new("bind-address")
                .long("bind-address")
                .takes_value(true)
                .value_parser(clap::value_parser!(SocketAddr))
                .help(BIND_ADDRESS_HELP)
                .long_help(BIND_ADDRESS_HELP_LONG),
        )
        .arg(
            Arg::new("doh-server")
                .long("doh-server")
                .action(ArgAction::Append)
                .takes_value(true)
                .value_parser(clap::value_parser!(DoHAddress))
                .default_values(&["1.1.1.1:443/cloudflare-dns.com", "8.8.8.8:443/google.dns"])
                .help(DOH_SERVER_HELP)
                .long_help(DOH_SERVER_HELP_LONG),
        )
        .subcommand(address_command)
        .subcommand(record_command)
}

pub async fn run(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("address", sub_matches)) => handle_address_command(arg_matches, sub_matches).await,
        Some(("record", sub_matches)) => handle_record_command(arg_matches, sub_matches).await,
        _ => unreachable!(),
    }
}

fn config_resolver(
    mut builder: ResolverBuilder,
    matches: &ArgMatches,
) -> anyhow::Result<ResolverBuilder> {
    match matches.get_many::<DoHAddress>("doh-server") {
        Some(values) => {
            for value in values {
                builder = builder.with_doh_server(value.0, &value.1);
            }
        }
        None => {}
    }

    match matches.get_one::<SocketAddr>("bind-address") {
        Some(value) => {
            builder = builder.with_bind_address(*value);
        }
        None => {}
    }

    Ok(builder)
}

async fn handle_address_command(
    matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let builder = config_resolver(Resolver::builder(), matches)?;
    let resolver = builder.build();
    let response = resolver
        .lookup_address(sub_matches.get_one::<String>("hostname").unwrap())
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

async fn handle_record_command(
    matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let builder = config_resolver(Resolver::builder(), matches)?;
    let resolver = builder.build();
    let response = resolver
        .lookup_record(
            sub_matches.get_one::<String>("type").unwrap(),
            sub_matches.get_one::<String>("hostname").unwrap(),
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
