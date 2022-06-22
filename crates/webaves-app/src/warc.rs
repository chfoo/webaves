use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use webaves::{
    compress::CompressionFormat,
    header::HeaderMap,
    warc::{HeaderMetadata, WARCReader, WARCWriter},
};

use crate::argutil::{MultiInput, OutputStream};

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
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help("Allow overwriting existing files."),
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

    let load_command = Command::new("load")
        .about("Transform JSON formatted input to WARC file")
        .arg(
            Arg::new("input")
                .required(true)
                .multiple_values(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to JSON file"),
        )
        .arg(
            Arg::new("compression_format")
                .long("compress")
                .value_parser(["none", "gzip", "zstd"])
                .default_value("none")
                .help("Apply compression to the output."),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .takes_value(true)
                .default_value("-")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to output WARC"),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .action(ArgAction::SetTrue)
                .help("Allow overwriting existing files."),
        );

    Command::new("warc")
        .about("Process WARC files.")
        .long_about("Read or manipulate WARC files")
        .subcommand_required(true)
        .subcommand(dump_command)
        .subcommand(list_command)
        .subcommand(load_command)
}

pub fn run(global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("dump", sub_matches)) => handle_dump_command(global_matches, sub_matches),
        Some(("list", sub_matches)) => handle_list_command(global_matches, sub_matches),
        Some(("load", sub_matches)) => handle_load_command(global_matches, sub_matches),
        _ => unreachable!(),
    }
}

fn read_warc_files_loop<FH, FB, FF>(
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
    let mut multi_input = MultiInput::from_args(global_matches, sub_matches)?;
    let mut output = OutputStream::from_args(sub_matches)?;

    let mut buffer = Vec::new();
    buffer.resize(16384, 0);

    while let Some(file) = multi_input.next_file()? {
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
                multi_input
                    .progress_bar
                    .set_position(block.raw_file_offset());
            }

            reader.end_record(block)?;
            footer_callback(&mut output)?;
        }
    }

    multi_input.progress_bar.finish_and_clear();

    Ok(())
}

fn handle_dump_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    read_warc_files_loop(
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

    read_warc_files_loop(
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

fn handle_load_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let compression_format = get_compression_format(sub_matches);
    let mut multi_input = MultiInput::from_args(global_matches, sub_matches)?;
    let output = OutputStream::from_args(sub_matches)?;
    let mut writer = WARCWriter::new_compressed(output, compression_format, Default::default());

    let mut buffer = Vec::new();
    buffer.resize(16384, 0);

    while let Some(file) = multi_input.next_file()? {
        let mut reader = BufReader::new(file);
        let mut line_buf = String::new();
        let mut block_writer = None;

        loop {
            line_buf.clear();
            let amount = reader.read_line(&mut line_buf)?;
            let line = line_buf.trim();

            if line.is_empty() {
                break;
            }

            let element = serde_json::from_str::<DumpElementOwned>(line)?;

            match element {
                DumpElementOwned::Header { version, fields } => {
                    anyhow::ensure!(block_writer.is_none());
                    writer.set_version(version);

                    writer.begin_record(&fields)?;
                    block_writer = Some(writer.write_block());
                }
                DumpElementOwned::Block { data } => {
                    anyhow::ensure!(block_writer.is_some());
                    block_writer.as_mut().unwrap().write_all(&data)?;
                }
                DumpElementOwned::EndOfRecord => {
                    anyhow::ensure!(block_writer.is_some());
                    writer.end_record(block_writer.take().unwrap())?;
                }
            }

            multi_input.progress_bar.inc(amount as u64);
        }
    }

    multi_input.progress_bar.finish_and_clear();

    Ok(())
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
