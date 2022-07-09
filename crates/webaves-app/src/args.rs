use std::net::SocketAddr;

use clap::{Arg, Command};

pub fn root_command<'h>() -> Command<'h> {
    let command = Command::new(clap::crate_name!())
        .about(crate::message::static_text("program-about"))
        .version(clap::crate_version!())
        .subcommand_required(true)
        .subcommand(Command::new("crash_error").hide(true))
        .subcommand(Command::new("crash_panic").hide(true))
        .subcommand(crate::dns_lookup::create_command())
        // .subcommand(crate::echo::create_server_command())
        .subcommand(crate::echo::create_client_command())
        .subcommand(crate::service::create_service_command())
        .subcommand(crate::warc::create_command());

    crate::logging::logging_args(command)
}

const BIND_ADDRESS_HELP: &str = "Address of the outgoing network interface";
const BIND_ADDRESS_HELP_LONG: &str = "IP address and port number of the outgoing network interface.

Example: \"192.168.1.100:0\" specifies the network interface \
with IP address 192.168.1.100, and 0 to indicate a default port number.";

pub fn bind_address<'h>() -> Arg<'h> {
    Arg::new("bind-address")
        .long("bind-address")
        .takes_value(true)
        .value_parser(clap::value_parser!(SocketAddr))
        .help(BIND_ADDRESS_HELP)
        .long_help(BIND_ADDRESS_HELP_LONG)
}
