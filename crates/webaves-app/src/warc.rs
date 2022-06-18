use std::{fs::File, io::Read, path::PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use webaves::{header::HeaderMap, warc::WARCReader};

pub fn create_command() -> Command<'static> {
    let dump_command = Command::new("dump")
        .about("Transform WARC files to JSON.")
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to WARC file."),
        )
        .arg(
            Arg::new("output")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to output file."),
        );
    let list_command = Command::new("list")
        .about("List WARC file contents by name-value fields.")
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to WARC file."),
        )
        .arg(
            Arg::new("output")
                .value_parser(clap::value_parser!(PathBuf))
                .default_value("-")
                .help("Path to output file."),
        )
        .arg(
            Arg::new("name")
                .long("name")
                .takes_value(true)
                .multiple_values(true)
                .help("Show values with the given field name."),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .conflicts_with("name")
                .help("Show all fields including the name."),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Format at the output as JSON"),
        )
        .arg(
            Arg::new("csv")
                .long("csv")
                .action(ArgAction::SetTrue)
                .help("Format at the output as CSV"),
        );

    Command::new("warc")
        .about("Process WARC files.")
        .subcommand_required(true)
        .subcommand(dump_command)
        .subcommand(list_command)
}

pub fn run(global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("dump", sub_matches)) => handle_dump_command(global_matches, sub_matches),
        _ => unreachable!(),
    }
}

fn handle_dump_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let paths = sub_matches
        .get_many::<PathBuf>("input")
        .unwrap()
        .collect::<Vec<&PathBuf>>();
    let total_file_size = get_total_file_size(&paths)?;

    let progress_bar = crate::logging::create_and_config_progress_bar(global_matches);
    progress_bar.set_length(total_file_size);

    let mut buffer = Vec::new();
    buffer.resize(16384, 0);

    for path in paths {
        let file = File::open(&path)?;
        let mut reader = WARCReader::new(file)?;

        loop {
            let metadata = reader.begin_record()?;

            if metadata.is_none() {
                break;
            }

            let metadata = metadata.unwrap();
            let metadata_string = serde_json::to_string(&DumpElement::Header {
                version: metadata.version(),
                fields: metadata.header(),
            })?;
            println!("{}", metadata_string);

            let mut block = reader.read_block();
            loop {
                let amount = block.read(&mut buffer)?;

                if amount == 0 {
                    break;
                }

                let block_string = serde_json::to_string(&DumpElement::Block {
                    data: &buffer[0..amount],
                })?;
                println!("{}", block_string);

                progress_bar.set_position(block.raw_file_offset());
            }

            reader.end_record(block)?;

            let end_string = serde_json::to_string(&DumpElement::EndOfRecord)?;
            println!("{}", end_string);
        }
    }

    progress_bar.finish_and_clear();

    Ok(())
}

fn get_total_file_size(paths: &[&PathBuf]) -> anyhow::Result<u64> {
    let mut total = 0;

    for path in paths {
        let metadata = std::fs::metadata(path)?;
        total += metadata.len();
    }

    Ok(total)
}

#[derive(Serialize)]
enum DumpElement<'a> {
    Header {
        version: &'a str,
        fields: &'a HeaderMap,
    },
    Block {
        data: &'a [u8],
    },
    EndOfRecord,
}

#[derive(Serialize, Deserialize)]
enum DumpElementOwned {
    Header { version: String, fields: HeaderMap },
    Block { data: Vec<u8> },
    EndOfRecord,
}
