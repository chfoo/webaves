use clap::{Arg, ArgMatches, Command};
use webaves::dns::{Resolver, ResolverBuilder};

pub async fn main() -> anyhow::Result<()> {
    let address_command = Command::new("address")
        .about("Lookup IP addresses for a hostname.")
        .arg(
            Arg::new("hostname")
                .required(true)
                .help("Target hostname to query."),
        );

    let record_command = Command::new("record")
        .about("Lookup records for a hostname.")
        .arg(
            Arg::new("type")
                .required(true)
                .help("DNS record type as string or integer."),
        )
        .arg(
            Arg::new("hostname")
                .required(true)
                .help("Target hostname to query."),
        );

    let command = Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about("Lookup DNS records")
        .subcommand_required(true)
        .arg(
            Arg::new("bind-address")
                .long("bind-address")
                .short('b')
                .takes_value(true)
                .help("Address of outgoing network interface. (Example: 192.168.1.100:0)"),
        )
        .arg(
            Arg::new("doh-server")
            .long("doh-server")
            .short('h')
                .multiple_occurrences(true)
                    .takes_value(true)
                .help("Address and hostname of DNS-over-HTTPS server. (Example: 10.0.0.0:443/dns.example.com)"),
        )
        .subcommand(address_command)
        .subcommand(record_command);

    let command = crate::logging::logging_args(command);
    let matches = command.get_matches();

    crate::logging::set_up_logging(&matches);

    match matches.subcommand() {
        Some(("address", sub_matches)) => handle_address_command(&matches, sub_matches).await,
        Some(("record", sub_matches)) => handle_record_command(&matches, sub_matches).await,
        _ => unreachable!(),
    }
}

fn config_resolver(
    mut builder: ResolverBuilder,
    matches: &ArgMatches,
) -> anyhow::Result<ResolverBuilder> {
    match matches.values_of("doh-server") {
        Some(values) => {
            for value in values {
                match value.split_once('/') {
                    Some((address, hostname)) => {
                        builder = builder.with_doh_server(address.parse()?, hostname);
                    }
                    None => anyhow::bail!("bad DOH address format"),
                }
            }
        }
        None => {
            builder = builder
                .with_doh_server("1.1.1.1:443".parse().unwrap(), "cloudflare-dns.com")
                .with_doh_server("8.8.8.8:443".parse().unwrap(), "google.dns");
        }
    }

    match matches.value_of("bind-address") {
        Some(value) => {
            builder = builder.with_bind_address(value.parse()?);
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
        .lookup_address(sub_matches.value_of("hostname").unwrap())
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
            sub_matches.value_of("type").unwrap(),
            sub_matches.value_of("hostname").unwrap(),
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
