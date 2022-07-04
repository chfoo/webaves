use std::{
    io::{Read, Write},
    path::Path,
};

use clap::ArgMatches;
use webaves::{
    io::SourceCountRead,
    warc::{HeaderMetadata, WARCReader},
};

use crate::argutil::{MultiInput, OutputStream};

pub fn read_warc_files_loop<FH, FB, FF>(
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
                let previous_offset = block.source_read_count();
                let amount = block.read(&mut buffer)?;

                if amount == 0 {
                    break;
                }

                body_callback(&mut output, &buffer, amount)?;
                multi_input
                    .progress_bar
                    .inc(block.source_read_count() - previous_offset);
            }

            reader.end_record()?;
            footer_callback(&mut output)?;
        }
    }

    multi_input.progress_bar.finish_and_clear();

    Ok(())
}

pub fn handle_list_command(
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
                match metadata.fields().get_str(name.as_str()) {
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
