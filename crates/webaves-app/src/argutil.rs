use std::{
    fs::File,
    io::{Read, Stdin, Stdout, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Context;
use clap::Command;

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
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        if path.as_ref().as_os_str() == "-" {
            Ok(Self::Stdout(std::io::stdout()))
        } else {
            Ok(Self::File(
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)?,
            ))
        }
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

pub fn build_commands() -> Command<'static> {
    let command = Command::new(clap::crate_name!())
        .about("Web archive software suite")
        .version(clap::crate_version!())
        .subcommand_required(true)
        .subcommand(Command::new("crash_error").hide(true))
        .subcommand(Command::new("crash_panic").hide(true))
        .subcommand(crate::dns_lookup::create_command())
        .subcommand(crate::echo::create_server_command())
        .subcommand(crate::echo::create_client_command())
        .subcommand(crate::warc::create_command());

    crate::logging::logging_args(command)
}

pub fn get_total_file_size(paths: &[&PathBuf]) -> anyhow::Result<u64> {
    let mut total = 0;

    for path in paths {
        if path.as_os_str() == "-" {
            continue;
        }

        let metadata = std::fs::metadata(path).context("failed get size of file")?;
        total += metadata.len();
    }

    Ok(total)
}
