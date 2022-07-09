use std::net::SocketAddr;

use clap::{Arg, ArgAction, ArgMatches};
use webaves::dns::ResolverBuilder;

use crate::argtypes::DoHAddress;

pub fn arg_doh_server<'h>() -> Arg<'h> {
    Arg::new("doh-server")
        .long("doh-server")
        .action(ArgAction::Append)
        .takes_value(true)
        .value_parser(clap::value_parser!(DoHAddress))
        .default_values(&["1.1.1.1:443/cloudflare-dns.com", "8.8.8.8:443/google.dns"])
        .help(crate::message::static_text("doh-server-help"))
        .long_help(crate::message::static_text("doh-server-help-long"))
}

pub fn config_resolver(
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
