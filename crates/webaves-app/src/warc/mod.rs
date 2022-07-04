mod dump;
mod extract;
mod read;

use std::path::PathBuf;

use clap::{Arg, ArgAction, ArgMatches, Command};

use webaves::compress::CompressionFormat;

const ABOUT: &str = "Process WARC files";
const ABOUT_LONG: &str = "Read, manipulate, or write WARC files and records";
const DUMP_ABOUT: &str = "Transform WARC files to JSON formatted output";
const LIST_ABOUT: &str = "Listing of file contents using header fields";
const LOAD_ABOUT: &str = "Transform JSON formatted input to WARC file";
const PACK_ABOUT: &str = "Repackages WARC files";
const PACK_ABOUT_LONG: &str = "Repackages WARC files by splitting or joining them.

This command can be used to recompress, split, and join WARC files.

Although it is safe to concatenate WARC files without the use of a WARC aware \
tool, recompression and splitting is not. When using compression, each record \
should be individually compressed (multistream). When splitting files, WARC \
consuming software may expect records such \"warcinfo\" to be first or \
\"request\" and \"response\" records to be in the same file. This command \
will attempt to automatically handle them";
const EXTRACT_ABOUT: &str = "Decode and extract documents to files";
const EXTRACT_ABOUT_LONG: &str = "Decode and extract documents to files.

This command will attempt to decode and extract as many documents as possible \
from response and resource records. By default, the files will be placed in \
directories similar to its original URL.

This command does *not* recreate a website for local browsing; this command \
is intended for use as an \"unzipping\" tool.";
const CHECKSUM_ABOUT :&str = "Verifies checksums";
const CHECKSUM_ABOUT_LONG :&str = "Verifies WARC record checksums.

This processes each WARC record for a 'WARC-Block-Digest' field. If the record \
includes this field, the checksum is computed for the record's block.

The output is formatted as the record's ID, a space, and one of 'ok', 'fail', \
or 'skip'.
";
const INPUT_WARC_FILE_HELP: &str = "Path to WARC file";
const INPUT_JSON_FILE_HELP: &str = "Path to JSON file";
const OUTPUT_FILE_HELP: &str = "Path to output file";
const OUTPUT_WARC_FILE_HELP: &str = "Path to output WARC file";
const OUTPUT_DIR_HELP: &str = "Path of directory to write files";
const OVERWRITE_HELP: &str = "Allow overwriting existing files";
const OUTPUT_COMPRESSION_FORMAT_HELP: &str = "Apply compression to the output";
const OUTPUT_AS_JSON_HELP: &str = "Format the output as JSON";
const SHOW_FIELD_WITH_NAME_HELP: &str = "Show values with the given field name";
const INCLUDE_FILE_HELP: &str = "Include filename and file position";
const ACCEPT_HELP: &str = "";
const ACCEPT_PATTERN_HELP: &str = "";
const REJECT_HELP: &str = "";
const REJECT_PATTERN_HELP: &str = "";

pub fn create_command() -> Command<'static> {
    let dump_command = Command::new("dump")
        .about(DUMP_ABOUT)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_FILE_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP),
        );
    let list_command = Command::new("list")
        .about(LIST_ABOUT)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_FILE_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP),
        )
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
                .help(SHOW_FIELD_WITH_NAME_HELP),
        )
        .arg(
            Arg::new("include_file")
                .long("include-file")
                .action(ArgAction::SetTrue)
                .help(INCLUDE_FILE_HELP),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help(OUTPUT_AS_JSON_HELP),
        );

    let load_command = Command::new("load")
        .about(LOAD_ABOUT)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_JSON_FILE_HELP),
        )
        .arg(
            Arg::new("compression_format")
                .long("compress")
                .value_parser(["none", "gzip", "zstd"])
                .default_value("none")
                .help(OUTPUT_COMPRESSION_FORMAT_HELP),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP),
        );

    let pack_command = Command::new("pack")
        .hide(true)
        .about(PACK_ABOUT)
        .long_about(PACK_ABOUT_LONG)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("compression_format")
                .long("compress")
                .value_parser(["none", "gzip", "zstd"])
                .default_value("none")
                .help(OUTPUT_COMPRESSION_FORMAT_HELP),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("output_directory")
                .long("output-directory")
                .short('d')
                .takes_value(true)
                .conflicts_with("output")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_DIR_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP),
        );

    let extract_command = Command::new("extract")
        .about(EXTRACT_ABOUT)
        .long_about(EXTRACT_ABOUT_LONG)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("output_directory")
                .long("output")
                .short('o')
                .takes_value(true)
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_DIR_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP)
                .hide(true),
        )
        .arg(
            Arg::new("accept")
                .long("accept")
                .takes_value(true)
                .multiple_values(true)
                .help(ACCEPT_HELP)
                .hide(true),
        )
        .arg(
            Arg::new("accept_pattern")
                .long("accept-pattern")
                .takes_value(true)
                .multiple_values(true)
                .help(ACCEPT_PATTERN_HELP)
                .hide(true),
        )
        .arg(
            Arg::new("reject")
                .long("reject")
                .takes_value(true)
                .multiple_values(true)
                .help(REJECT_HELP)
                .hide(true),
        )
        .arg(
            Arg::new("reject_pattern")
                .long("reject-pattern")
                .takes_value(true)
                .multiple_values(true)
                .help(REJECT_PATTERN_HELP)
                .hide(true),
        );

    let checksum_command = Command::new("checksum")
        .about(CHECKSUM_ABOUT)
        .long_about(CHECKSUM_ABOUT_LONG)
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(INPUT_WARC_FILE_HELP),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help(OUTPUT_FILE_HELP),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help(OVERWRITE_HELP),
        );

    Command::new("warc")
        .about(ABOUT)
        .long_about(ABOUT_LONG)
        .subcommand_required(true)
        .subcommand(dump_command)
        .subcommand(list_command)
        .subcommand(load_command)
        .subcommand(pack_command)
        .subcommand(extract_command)
        .subcommand(checksum_command)
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
