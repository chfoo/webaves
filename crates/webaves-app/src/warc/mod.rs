mod dump;
mod extract;
mod read;

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};

use webaves::compress::CompressionFormat;

pub fn create_command<'h>() -> Command<'h> {
    let dump_command = Command::new("dump")
        .about(crate::message::static_text("warc-dump-about"))
        .arg(input_warc_file_arg())
        .arg(output_file_arg())
        .arg(allow_overwrite_arg());
    let list_command = Command::new("list")
        .about(crate::message::static_text("warc-list-about"))
        .arg(input_warc_file_arg())
        .arg(output_file_arg())
        .arg(allow_overwrite_arg())
        .arg(
            Arg::new("name")
                .long("name")
                .takes_value(true)
                .action(ArgAction::Append)
                .default_values(&[
                    "WARC-Date",
                    "WARC-Type",
                    "Content-Type",
                    "Content-Length",
                    "WARC-Target-URI",
                ])
                .help(crate::message::static_text(
                    "warc-list-show-field-with-name-help",
                )),
        )
        .arg(
            Arg::new("include_file")
                .long("include-file")
                .action(ArgAction::SetTrue)
                .help(crate::message::static_text("warc-list-include-file-help")),
        )
        .arg(output_as_json_arg());

    let load_command = Command::new("load")
        .about(crate::message::static_text("warc-load-about"))
        .arg(input_json_file_arg())
        .arg(compression_format_arg())
        .arg(output_warc_file_arg())
        .arg(allow_overwrite_arg());

    let pack_command = Command::new("pack")
        .hide(true)
        .about(crate::message::static_text("warc-pack-about"))
        .long_about(crate::message::static_text("warc-pack-about-long"))
        .arg(input_warc_file_arg())
        .arg(compression_format_arg())
        .arg(output_warc_file_arg())
        .arg(output_dir_arg().conflicts_with("output"))
        .arg(allow_overwrite_arg());

    let extract_command = Command::new("extract")
        .about(crate::message::static_text("warc-extract-about"))
        .long_about(crate::message::static_text("warc-extract-about-long"))
        .arg(input_warc_file_arg())
        .arg(output_dir_arg())
        .arg(allow_overwrite_arg().hide(true))
        .arg(
            Arg::new("accept")
                .long("accept")
                .takes_value(true)
                .multiple_values(true)
                .hide(true),
        )
        .arg(
            Arg::new("accept_pattern")
                .long("accept-pattern")
                .takes_value(true)
                .multiple_values(true)
                .hide(true),
        )
        .arg(
            Arg::new("reject")
                .long("reject")
                .takes_value(true)
                .multiple_values(true)
                .hide(true),
        )
        .arg(
            Arg::new("reject_pattern")
                .long("reject-pattern")
                .takes_value(true)
                .multiple_values(true)
                .hide(true),
        );

    let checksum_command = Command::new("checksum")
        .about(crate::message::static_text("warc-checksum-about"))
        .long_about(crate::message::static_text("warc-checksum-about-long"))
        .arg(input_warc_file_arg())
        .arg(output_file_arg())
        .arg(allow_overwrite_arg());

    Command::new("warc")
        .about(crate::message::static_text("warc-about"))
        .long_about(crate::message::static_text("warc-about-long"))
        .subcommand_required(true)
        .subcommand(dump_command)
        .subcommand(list_command)
        .subcommand(load_command)
        .subcommand(pack_command)
        .subcommand(extract_command)
        .subcommand(checksum_command)
}

fn input_warc_file_arg<'h>() -> Arg<'h> {
    Arg::new("input")
        .required(true)
        .multiple_values(true)
        .value_parser(clap::value_parser!(PathBuf))
        .help(crate::message::static_text("input-warc-file-help"))
}

fn input_json_file_arg<'h>() -> Arg<'h> {
    Arg::new("input")
        .required(true)
        .multiple_values(true)
        .value_parser(clap::value_parser!(PathBuf))
        .help(crate::message::static_text("input-json-file-help"))
}

fn output_file_arg<'h>() -> Arg<'h> {
    Arg::new("output")
        .long("output")
        .short('o')
        .takes_value(true)
        .default_value("-")
        .value_parser(clap::value_parser!(PathBuf))
        .help(crate::message::static_text("output-file-help"))
}

fn output_warc_file_arg<'h>() -> Arg<'h> {
    Arg::new("output")
        .long("output")
        .short('o')
        .takes_value(true)
        .default_value("-")
        .value_parser(clap::value_parser!(PathBuf))
        .help(crate::message::static_text("output-warc-file-help"))
}

fn output_dir_arg<'h>() -> Arg<'h> {
    Arg::new("output_directory")
        .long("output-directory")
        .short('d')
        .takes_value(true)
        .value_parser(clap::value_parser!(PathBuf))
        .help(crate::message::static_text("output-dir-help"))
}

fn output_as_json_arg<'h>() -> Arg<'h> {
    Arg::new("json")
        .long("json")
        .action(ArgAction::SetTrue)
        .help(crate::message::static_text("output-as-json-help"))
}

fn allow_overwrite_arg<'h>() -> Arg<'h> {
    Arg::new("overwrite")
        .long("overwrite")
        .action(ArgAction::SetTrue)
        .help(crate::message::static_text("allow-overwrite-help"))
}

fn compression_format_arg<'h>() -> Arg<'h> {
    Arg::new("compression_format")
        .long("compress")
        .value_parser(["none", "gzip", "zstd"])
        .default_value("none")
        .help(crate::message::static_text(
            "output-compression-format-help",
        ))
}

pub fn run(global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("dump", sub_matches)) => dump::handle_dump_command(global_matches, sub_matches),
        Some(("list", sub_matches)) => read::handle_list_command(global_matches, sub_matches),
        Some(("load", sub_matches)) => dump::handle_load_command(global_matches, sub_matches),
        Some(("pack", _sub_matches)) => todo!(),
        Some(("extract", sub_matches)) => {
            extract::handle_extract_command(global_matches, sub_matches)
        }
        Some(("checksum", sub_matches)) => {
            read::handle_checksum_command(global_matches, sub_matches)
        }
        _ => unreachable!(),
    }
}

fn get_compression_format(arg_matches: &ArgMatches) -> CompressionFormat {
    match arg_matches
        .get_one::<String>("compression_format")
        .unwrap()
        .as_str()
    {
        "none" => CompressionFormat::Raw,
        "gzip" => CompressionFormat::Gzip,
        "zstd" => CompressionFormat::Zstd,
        _ => unreachable!(),
    }
}
