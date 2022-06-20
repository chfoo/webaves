use std::net::SocketAddr;

use clap::{Arg, ArgAction, ArgMatches, Command};
use webaves::dns::{Resolver, ResolverBuilder};

use crate::argutil::DoHAddress;

pub fn create_command() -> Command<'static> {
    let address_command = Command::new("address")
        .about("Lookup IP addresses for a hostname")
        .arg(
            Arg::new("hostname")
                .required(true)
                .help("Target hostname to query"),
        );

    let record_command = Command::new("record")
        .about("Lookup records for a hostname")
        .arg(
            Arg::new("type")
                .required(true)
                .help("DNS record type as string or integer"),
        )
        .arg(
            Arg::new("hostname")
                .required(true)
                .help("Target hostname to query"),
        );

    Command::new("dns-lookup")
        .about("Lookup DNS records")
        .subcommand_required(true)
        .arg(
            Arg::new("bind-address")
                .long("bind-address")
                .takes_value(true)
                .value_parser(clap::value_parser!(SocketAddr))
                .help("Address of outgoing network interface. (Example: 192.168.1.100:0)"),
        )
        .arg(
            Arg::new("doh-server")
                .long("doh-server")
                .action(ArgAction::Append)
                .takes_value(true)
                .value_parser(clap::value_parser!(DoHAddress))
                .help("Address and hostname of DNS-over-HTTPS server. (Example: 10.0.0.0:443/dns.example.com)"),
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
        None => {
            builder = builder
                .with_doh_server("1.1.1.1:443".parse().unwrap(), "cloudflare-dns.com")
                .with_doh_server("8.8.8.8:443".parse().unwrap(), "google.dns");
        }
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
