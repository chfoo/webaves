use std::io::{BufRead, BufReader, Write};

use clap::ArgMatches;
use serde::{Deserialize, Serialize};
use webaves::{header::HeaderMap, warc::WARCWriter};

use crate::argutil::{MultiInput, OutputStream};

use super::read::read_warc_files_loop;

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

pub fn handle_dump_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    read_warc_files_loop(
        global_matches,
        sub_matches,
        |_input_path, output, metadata| {
            let metadata_string = serde_json::to_string(&DumpElement::Header {
                version: metadata.version(),
                fields: metadata.fields(),
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

pub fn handle_load_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let compression_format = super::get_compression_format(sub_matches);
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
