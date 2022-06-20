use std::{
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Context;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use webaves::{
    header::HeaderMap,
    warc::{HeaderMetadata, WARCReader},
};

use crate::argutil::{InputStream, OutputStream};

pub fn create_command() -> Command<'static> {
    let dump_command = Command::new("dump")
        .about("Transform WARC files to JSON formatted output")
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to WARC file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to output file"),
        );
    let list_command = Command::new("list")
        .about("Listing of file contents using header fields")
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to WARC file"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to output file"),
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
                .help("Show values with the given field name"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Format at the output as JSON"),
        );

    Command::new("warc")
        .about("Process WARC files.")
        .long_about("Read or manipulate WARC files")
        .subcommand_required(true)
        .subcommand(dump_command)
        .subcommand(list_command)
}

pub fn run(global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("dump", sub_matches)) => handle_dump_command(global_matches, sub_matches),
        Some(("list", sub_matches)) => handle_list_command(global_matches, sub_matches),
        _ => unreachable!(),
    }
}

fn read_files_loop<FH, FB, FF>(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
    mut header_callback: FH,
    mut body_callback: FB,
    mut footer_callback: FF,
) -> anyhow::Result<()>
where
    FH: FnMut(&mut OutputStream, &HeaderMetadata) -> anyhow::Result<()>,
    FB: FnMut(&mut OutputStream, &[u8], usize) -> anyhow::Result<()>,
    FF: FnMut(&mut OutputStream) -> anyhow::Result<()>,
{
    let paths = sub_matches
        .get_many::<PathBuf>("input")
        .unwrap()
        .collect::<Vec<&PathBuf>>();
    let total_file_size = crate::argutil::get_total_file_size(&paths)?;
    let output = sub_matches.get_one::<PathBuf>("output").unwrap();
    let mut output = OutputStream::open(output).context("failed to create file")?;

    let progress_bar = crate::logging::create_and_config_progress_bar(global_matches);
    progress_bar.set_length(total_file_size);

    let mut buffer = Vec::new();
    buffer.resize(16384, 0);

    for path in paths {
        tracing::info!(?path, "reading file");
        let file = InputStream::open(path).context("failed to open file")?;
        let mut reader = WARCReader::new(file)?;

        loop {
            let metadata = reader.begin_record()?;

            if metadata.is_none() {
                break;
            }

            let metadata = metadata.unwrap();
            header_callback(&mut output, &metadata)?;

            let mut block = reader.read_block();
            loop {
                let amount = block.read(&mut buffer)?;

                if amount == 0 {
                    break;
                }

                body_callback(&mut output, &buffer, amount)?;
                progress_bar.set_position(block.raw_file_offset());
            }

            reader.end_record(block)?;
            footer_callback(&mut output)?;
        }
    }

    progress_bar.finish_and_clear();

    Ok(())
}

fn handle_dump_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    read_files_loop(
        global_matches,
        sub_matches,
        |output, metadata| {
            let metadata_string = serde_json::to_string(&DumpElement::Header {
                version: metadata.version(),
                fields: metadata.header(),
            })?;
            output.write_all(metadata_string.as_bytes())?;
            output.write_all(b"\n")?;
            Ok(())
        },
        |output, buffer, amount| {
            let block_string = serde_json::to_string(&DumpElement::Block {
                data: &buffer[0..amount],
            })?;
            output.write_all(block_string.as_bytes())?;
            output.write_all(b"\n")?;

            Ok(())
        },
        |output| {
            let end_string = serde_json::to_string(&DumpElement::EndOfRecord)?;
            output.write_all(end_string.as_bytes())?;
            output.write_all(b"\n")?;
            Ok(())
        },
    )
}

fn handle_list_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let names = sub_matches
        .get_many::<String>("name")
        .unwrap()
        .collect::<Vec<&String>>();
    let is_json = sub_matches.get_one::<bool>("json").cloned().unwrap();

    read_files_loop(
        global_matches,
        sub_matches,
        |output, metadata| {
            let mut line_buffer = Vec::new();

            for name in &names {
                match metadata.header().get_str(name.as_str()) {
                    Some(value) => line_buffer.push(value),
                    None => line_buffer.push(""),
                }
            }

            if is_json {
                output.write_all(serde_json::to_string(&line_buffer)?.as_bytes())?;
                output.write_all(b"\n")?;
            } else {
                let mut writer = csv::Writer::from_writer(Vec::new());
                writer.serialize(&line_buffer)?;
                output.write_all(&writer.into_inner()?)?;
            }

            Ok(())
        },
        |_output, _buffer, _amount| Ok(()),
        |_output| Ok(()),
    )
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
