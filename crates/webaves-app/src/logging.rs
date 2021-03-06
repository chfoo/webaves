use std::{
    fs::File,
    io::{BufWriter, Write},
    panic::PanicInfo,
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};

use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tracing_subscriber::{prelude::*, EnvFilter};

const PROGRESS_BAR_STYLE_TEMPLATE: &str =
    r"[{elapsed_precise}] {wide_bar} {percent:>3}% {bytes}/{total_bytes}";

lazy_static::lazy_static! {
    static ref PROGRESS_BAR: RwLock<Option<ProgressBar>> =RwLock::new(None);
}

pub struct LogLineWriter {
    file: Option<BufWriter<File>>,
}

impl LogLineWriter {
    pub fn new() -> Self {
        Self { file: None }
    }

    pub fn open_file(path: &Path) -> Result<Self, std::io::Error> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self {
            file: Some(BufWriter::new(file)),
        })
    }
}

impl Write for LogLineWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.file {
            Some(file) => file.write(buf),
            None => match progress_bar() {
                Some(progress_bar) => progress_bar.suspend(|| std::io::stderr().write(buf)),
                None => std::io::stderr().write(buf),
            },
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.file {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

pub fn logging_args(command: Command) -> Command {
    command
        .arg(
            Arg::new("log_level")
                .long("log-level")
                .short('l')
                .value_parser(["error", "warn", "info", "debug", "trace"])
                .default_value("warn")
                .default_value_if("verbose", None, Some("info"))
                .help(crate::message::static_text("log-level-help")),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help(crate::message::static_text("verbose-help"))
                .help(crate::message::static_text("verbose-help-long")),
        )
        .arg(
            Arg::new("log_filter")
                .long("log-filter")
                .conflicts_with("log_level")
                .takes_value(true)
                .help(crate::message::static_text("log-filter-help")),
        )
        .arg(
            Arg::new("log_file")
                .long("log-file")
                .takes_value(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help(crate::message::static_text("log-file-help")),
        )
        .arg(
            Arg::new("log_format")
                .long("log-format")
                .value_parser(["default", "json"])
                .default_value("default")
                .help(crate::message::static_text("log-format-help"))
                .long_help(crate::message::static_text("log-format-help-long")),
        )
}

pub fn set_up_logging(arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let filter = if arg_matches.contains_id("log_filter") {
        EnvFilter::try_from(arg_matches.get_one::<String>("log_filter").unwrap())?
    } else {
        let level = arg_matches.get_one::<String>("log_level").unwrap();
        EnvFilter::try_from(format!("webaves={},webaves_app={}", level, level))?
    };

    let mut ansi = use_console_color_stderr();
    let mut subscriber_default = None;
    let mut subscriber_json = None;

    let writer = Mutex::new(match arg_matches.get_one::<PathBuf>("log_file") {
        Some(path) => {
            ansi = false;
            LogLineWriter::open_file(path)?
        }
        None => LogLineWriter::new(),
    });

    match arg_matches
        .get_one::<String>("log_format")
        .unwrap()
        .as_str()
    {
        "default" => {
            subscriber_default = Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_ansi(ansi),
            )
        }
        "json" => {
            subscriber_json = Some(
                tracing_subscriber::fmt::layer()
                    .with_writer(writer)
                    .with_ansi(ansi)
                    .json(),
            )
        }
        _ => unreachable!(),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(subscriber_default)
        .with(subscriber_json)
        .init();

    set_up_panic_logging();

    Ok(())
}

fn set_up_panic_logging() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info: &PanicInfo| {
        tracing::error!(message = ?panic_info, "panic");
        original_hook(panic_info);
    }));
}

pub fn progress_bar() -> Option<ProgressBar> {
    PROGRESS_BAR.read().unwrap().clone()
}

pub fn set_progress_bar(value: Option<ProgressBar>) {
    let mut guard = PROGRESS_BAR.write().unwrap();
    match value {
        Some(value) => {
            guard.replace(value);
        }
        None => {
            guard.take();
        }
    }
}

pub fn is_verbose(arg_matches: &ArgMatches) -> bool {
    arg_matches.get_one::<bool>("verbose").cloned().unwrap()
}

pub fn create_and_config_progress_bar(arg_matches: &ArgMatches) -> ProgressBar {
    if is_verbose(arg_matches) {
        let progress_bar =
            ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr_with_hz(2));
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(PROGRESS_BAR_STYLE_TEMPLATE)
                .unwrap(),
        );
        set_progress_bar(Some(progress_bar.clone()));
        progress_bar
    } else {
        ProgressBar::hidden()
    }
}

#[allow(dead_code)]
pub fn use_console_color() -> bool {
    console::colors_enabled() && std::env::var_os("NO_COLOR").is_none()
}

pub fn use_console_color_stderr() -> bool {
    console::colors_enabled_stderr() && std::env::var_os("NO_COLOR").is_none()
}
