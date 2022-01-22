mod app;
mod dns;
mod error;
mod http;
mod io;
mod uuid;

use std::path::Path;

use clap::{App, Arg, ArgMatches};

fn main() -> anyhow::Result<()> {
    let app_args = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .author(clap::crate_authors!())
        .subcommand(run_args())
        .subcommand(new_config_args());
    let matches = app_args.get_matches();

    match matches.subcommand() {
        Some((name, sub_matches)) => process_subcommand(name, sub_matches),
        None => {
            unreachable!();
        }
    }
}

fn run_args() -> App<'static> {
    App::new("run")
        .about("Run a capture and archive project.")
        .arg(
            Arg::new("config")
                .help("Path of the project configuration file.")
                .required(true)
                .allow_invalid_utf8(true),
        )
}

fn new_config_args() -> App<'static> {
    App::new("new-config")
        .about("Create a new project configuration file.")
        .arg(
            Arg::new("path")
                .help("Path to where the file will be written.")
                .required(true)
                .allow_invalid_utf8(true),
        )
}

fn process_subcommand(name: &str, matches: &ArgMatches) -> anyhow::Result<()> {
    match name {
        "run" => self::app::run(Path::new(matches.value_of_os("config").unwrap())),
        "new-config" => self::app::new_config(Path::new(matches.value_of_os("path").unwrap())),
        _ => unreachable!(),
    }
}
