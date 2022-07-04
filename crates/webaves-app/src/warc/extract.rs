use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use clap::ArgMatches;
use indicatif::ProgressBar;
use url::Url;
use webaves::{
    io::SourceCountRead,
    warc::{extract::ExtractorDispatcher, BlockReader, HeaderMapExt, WARCReader},
};

use crate::argutil::MultiInput;

pub fn handle_extract_command(
    global_matches: &ArgMatches,
    sub_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let mut multi_input = MultiInput::from_args(global_matches, sub_matches)?;
    let output_dir = sub_matches.get_one::<PathBuf>("output_directory").unwrap();

    while let Some((_path, file)) = multi_input.next_file()? {
        let mut reader = WARCReader::new(file)?;

        loop {
            let has_more =
                process_extract_record(&multi_input.progress_bar, &mut reader, output_dir)?;

            if !has_more {
                break;
            }
        }
    }

    multi_input.progress_bar.finish_and_clear();

    Ok(())
}

fn process_extract_record<'a, 'b, R: Read>(
    progress_bar: &ProgressBar,
    reader: &'b mut WARCReader<'a, R>,
    output_dir: &Path,
) -> anyhow::Result<bool> {
    let metadata = reader.begin_record()?;

    if metadata.is_none() {
        return Ok(false);
    }

    let mut buf = Vec::new();
    buf.resize(16384, 0);

    let metadata = metadata.unwrap();

    let block_reader = reader.read_block();
    let mut extractor = ExtractorDispatcher::new(block_reader);
    extractor.add_default_extractors();
    let url = metadata.fields().get_parsed::<Url>("WARC-Target-URI")?;

    if extractor.can_accept_any(&metadata) && url.is_some() {
        let url = url.as_ref().unwrap();
        tracing::debug!(%url, "extractor begin");
        extractor.begin(&metadata)?;
        extract_record_with_extractor(url, output_dir, extractor, progress_bar)?;
    } else {
        let mut block_reader = extractor.into_inner();
        extract_record_nothing(&mut block_reader, progress_bar)?;
    }

    reader.end_record()?;

    Ok(true)
}

fn extract_record_with_extractor<'a, 's, R: Read>(
    url: &Url,
    output_dir: &Path,
    mut extractor: ExtractorDispatcher<'a, BlockReader<'a, 's, R>>,
    progress_bar: &ProgressBar,
) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    buf.resize(16384, 0);

    let temp_path = output_dir.join(format!("{}.tmp", webaves::uuid::new_v7().as_hyphenated()));
    let path = output_dir.join(webaves::download::url_to_path_buf(url));
    let path = webaves::download::remove_path_conflict(path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    tracing::info!(?path, %url, "extracting file");

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)?;

    loop {
        let previous_offset = extractor.get_ref().source_read_count();
        let amount = extractor.read(&mut buf)?;

        if amount == 0 {
            break;
        }

        file.write_all(&buf[0..amount])?;

        let current_offset = extractor.get_ref().source_read_count();
        progress_bar.inc(current_offset - previous_offset);
    }

    extractor.finish()?;

    std::fs::rename(temp_path, path)?;

    Ok(())
}

fn extract_record_nothing<R: Read>(
    block_reader: &mut BlockReader<R>,
    progress_bar: &ProgressBar,
) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    buf.resize(16384, 0);

    let mut previous_offset = block_reader.source_read_count();

    loop {
        let amount = block_reader.read(&mut buf)?;

        if amount == 0 {
            break;
        }

        progress_bar.inc(block_reader.source_read_count() - previous_offset);
        previous_offset = block_reader.source_read_count();
    }

    Ok(())
}
