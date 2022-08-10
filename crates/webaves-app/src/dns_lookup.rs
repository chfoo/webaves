use clap::{Arg, ArgMatches, Command};
use webaves::dns::Resolver;

pub fn create_command<'h>() -> Command<'h> {
    let address_command = Command::new("address")
        .about(crate::message::static_text("dns-lookup-address-about"))
        .arg(
            Arg::new("hostname")
                .required(true)
                .help(crate::message::static_text("dns-lookup-record-about")),
        );

    let record_command = Command::new("record")
        .about(crate::message::static_text("dns-lookup-record-about"))
        .arg(
            Arg::new("type")
                .required(true)
                .help(crate::message::static_text("dns-lookup-record-type-help")),
        )
        .arg(
            Arg::new("hostname")
                .required(true)
                .help(crate::message::static_text("dns-lookup-hostname-help")),
        );

    Command::new("dns-lookup")
        .about(crate::message::static_text("dns-lookup-about"))
        .long_about(crate::message::static_text("dns-lookup-about-long"))
        .subcommand_required(true)
        .arg(crate::args::bind_address())
        .arg(crate::dns::arg_doh_server())
        .subcommand(address_command)
        .subcommand(record_command)
}

pub fn run(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("address", sub_matches)) => handle_address_command(arg_matches, sub_matches),
        Some(("record", sub_matches)) => handle_record_command(arg_matches, sub_matches),
        _ => unreachable!(),
    }
}

fn handle_address_command(matches: &ArgMatches, sub_matches: &ArgMatches) -> anyhow::Result<()> {
    let builder = crate::dns::config_resolver(Resolver::builder(), matches)?;
    let resolver = builder.build();
    let response = resolver.lookup_address(sub_matches.get_one::<String>("hostname").unwrap())?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

fn handle_record_command(matches: &ArgMatches, sub_matches: &ArgMatches) -> anyhow::Result<()> {
    let builder = crate::dns::config_resolver(Resolver::builder(), matches)?;
    let resolver = builder.build();
    let response = resolver.lookup_record(
        sub_matches.get_one::<String>("type").unwrap(),
        sub_matches.get_one::<String>("hostname").unwrap(),
    )?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
