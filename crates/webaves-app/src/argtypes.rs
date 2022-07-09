use std::{
    collections::VecDeque,
    fs::File,
    io::{Read, Stdin, Stdout, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use clap::ArgMatches;
use indicatif::ProgressBar;

#[derive(Clone, Debug)]
pub struct DoHAddress(pub SocketAddr, pub String);

impl FromStr for DoHAddress {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once('/') {
            Some((address, hostname)) => {
                let address = address
                    .parse::<SocketAddr>()
                    .map_err(|error| error.to_string())?;
                Ok(DoHAddress(address, hostname.to_string()))
            }
            None => Err("bad DoH address format".to_string()),
        }
    }
}

pub enum InputStream {
    File(File),
    Stdin(Stdin),
}

impl InputStream {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        if path.as_ref().as_os_str() == "-" {
            Ok(Self::Stdin(std::io::stdin()))
        } else {
            Ok(Self::File(std::fs::File::open(path)?))
        }
    }
}

impl Read for InputStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            InputStream::File(s) => s.read(buf),
            InputStream::Stdin(s) => s.read(buf),
        }
    }
}

pub enum OutputStream {
    File(File),
    Stdout(Stdout),
}

impl OutputStream {
    pub fn open<P: AsRef<Path>>(path: P, overwrite: bool) -> std::io::Result<Self> {
        if path.as_ref().as_os_str() == "-" {
            Ok(Self::Stdout(std::io::stdout()))
        } else {
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true);

            if overwrite {
                opts.create(true);
            } else {
                opts.create_new(true);
            }

            Ok(Self::File(opts.open(path)?))
        }
    }

    pub fn from_args(sub_matches: &ArgMatches) -> anyhow::Result<Self> {
        let path = sub_matches.get_one::<PathBuf>("output").unwrap();
        let overwrite = sub_matches
            .get_one::<bool>("overwrite")
            .cloned()
            .unwrap_or_default();
        let output = OutputStream::open(&path, overwrite)
            .with_context(|| format!("failed to create file {path:?}"))?;
        Ok(output)
    }
}

impl Write for OutputStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            OutputStream::File(s) => s.write(buf),
            OutputStream::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            OutputStream::File(s) => s.flush(),
            OutputStream::Stdout(s) => s.flush(),
        }
    }
}

pub struct MultiInput {
    pub input_paths: Vec<PathBuf>,
    pub total_input_file_size: u64,
    pub progress_bar: ProgressBar,
    pending_paths: VecDeque<PathBuf>,
}

impl MultiInput {
    pub fn from_args(
        global_matches: &ArgMatches,
        sub_matches: &ArgMatches,
    ) -> anyhow::Result<Self> {
        let input_paths = sub_matches
            .get_many::<PathBuf>("input")
            .unwrap()
            .cloned()
            .collect::<Vec<PathBuf>>();

        let total_input_file_size = get_total_file_size(&input_paths)?;
        let progress_bar = crate::logging::create_and_config_progress_bar(global_matches);
        progress_bar.set_length(total_input_file_size);

        Ok(Self {
            pending_paths: VecDeque::from(input_paths.clone()),
            input_paths,
            total_input_file_size,
            progress_bar,
        })
    }

    pub fn next_file(&mut self) -> anyhow::Result<Option<(PathBuf, InputStream)>> {
        match self.pending_paths.pop_front() {
            Some(path) => {
                tracing::info!(?path, "reading file");
                let file = InputStream::open(&path)
                    .with_context(|| format!("failed to open file {path:?}"))?;
                Ok(Some((path, file)))
            }
            None => Ok(None),
        }
    }
}

fn get_total_file_size(paths: &[PathBuf]) -> anyhow::Result<u64> {
    let mut total = 0;

    for path in paths {
        if path.as_os_str() == "-" {
            continue;
        }

        let metadata =
            std::fs::metadata(path).with_context(|| format!("failed get size of file {path:?}"))?;
        total += metadata.len();
    }

    Ok(total)
}
