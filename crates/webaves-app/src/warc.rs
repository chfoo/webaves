use std::{
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use webaves::{
    compress::CompressionFormat,
    header::HeaderMap,
    warc::{HeaderMetadata, WARCReader, WARCWriter},
};

use crate::argutil::{MultiInput, OutputStream};

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
        )
        .arg(
            Arg::new("accept")
                .long("accept")
                .takes_value(true)
                .multiple_values(true)
                .help(ACCEPT_HELP),
        )
        .arg(
            Arg::new("accept_pattern")
                .long("accept-pattern")
                .takes_value(true)
                .multiple_values(true)
                .help(ACCEPT_PATTERN_HELP),
        )
        .arg(
            Arg::new("reject")
                .long("reject")
                .takes_value(true)
                .multiple_values(true)
                .help(REJECT_HELP),
        )
        .arg(
            Arg::new("reject_pattern")
                .long("reject-pattern")
                .takes_value(true)
                .multiple_values(true)
                .help(REJECT_PATTERN_HELP),
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
}

pub fn run(global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("dump", sub_matches)) => handle_dump_command(global_matches, sub_matches),
        Some(("list", sub_matches)) => handle_list_command(global_matches, sub_matches),
        Some(("load", sub_matches)) => handle_load_command(global_matches, sub_matches),
        Some(("pack", _sub_matches)) => todo!(),
        Some(("extract", _sub_matches)) => todo!(),
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
    FH: FnMut(&Path, &mut OutputStream, &HeaderMetadata) -> anyhow::Result<()>,
    FB: FnMut(&mut OutputStream, &[u8], usize) -> anyhow::Result<()>,
    FF: FnMut(&mut OutputStream) -> anyhow::Result<()>,
{
    let mut multi_input = MultiInput::from_args(global_matches, sub_matches)?;
    let mut output = OutputStream::from_args(sub_matches)?;

    let mut buffer = Vec::new();
    buffer.resize(16384, 0);

    while let Some((path, file)) = multi_input.next_file()? {
        let mut reader = WARCReader::new(file)?;

        loop {
            let metadata = reader.begin_record()?;

            if metadata.is_none() {
                break;
            }

            let metadata = metadata.unwrap();
            header_callback(&path, &mut output, &metadata)?;

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

            reader.end_record()?;
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
        |_input_path, output, metadata| {
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
    let include_file = sub_matches
        .get_one::<bool>("include_file")
        .cloned()
        .unwrap();

    read_warc_files_loop(
        global_matches,
        sub_matches,
        |input_path, output, metadata| {
            let mut line_buffer = Vec::new();

            if include_file {
                line_buffer.push(input_path.to_string_lossy().into_owned());
                line_buffer.push(metadata.raw_file_offset().to_string());
            }

            for name in &names {
                match metadata.header().get_str(name.as_str()) {
                    Some(value) => line_buffer.push(value.to_string()),
                    None => line_buffer.push("".to_string()),
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

    while let Some((_path, file)) = multi_input.next_file()? {
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
                    block_writer = None;
                    writer.end_record()?;
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
