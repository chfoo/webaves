use clap::{Arg, ArgMatches, Command};
use tracing_subscriber::{prelude::*, EnvFilter};

pub fn logging_args(command: Command) -> Command {
    command
        .arg(
            Arg::new("log_filter")
                .long("log-filter")
                .short('l')
                .help("Filter level of severity and targets of logging messages.")
                .default_value("warn"),
        )
        .arg(
            Arg::new("log_sink")
                .long("log-sink")
                .help("Destination of logging messages.")
                .possible_values(["stderr"])
                .default_value("stderr"),
        )
}

pub fn set_up_logging(arg_matches: &ArgMatches) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from(arg_matches.value_of("log_filter").unwrap()).unwrap())
        .init();
}
