use anyhow::Context;

mod argutil;
mod dns_lookup;
mod echo;
mod logging;
mod warc;

fn main() {
    let exit_code = main_inner();
    std::process::exit(exit_code);
}

#[tokio::main]
async fn main_inner() -> i32 {
    let result = main_inner_inner().await;

    match result {
        Ok(_) => {
            tracing::info!("program exit ok");
            0
        }
        Err(error) => {
            let error_message_line = format!("{:#}", error);
            let backtrace = format!("{}", error.backtrace());
            tracing::error!(error = %error_message_line, %backtrace, "program exit error");
            eprintln!("{error_message_line}");
            1
        }
    }
}

async fn main_inner_inner() -> anyhow::Result<()> {
    let command = crate::argutil::build_commands();
    let arg_matches = command.get_matches();

    crate::logging::set_up_logging(&arg_matches)?;

    match arg_matches.subcommand() {
        Some(("crash_error", _sub_matches)) => do_crash_error(),
        Some(("crash_panic", _sub_matches)) => do_crash_panic(),
        Some(("dns-lookup", sub_matches)) => crate::dns_lookup::run(sub_matches).await,
        Some(("echo-service", sub_matches)) => crate::echo::run_server(sub_matches).await,
        Some(("echo", sub_matches)) => crate::echo::run_client(&arg_matches, sub_matches).await,
        Some(("warc", sub_matches)) => crate::warc::run(&arg_matches, sub_matches),
        _ => unreachable!(),
    }?;

    Ok(())
}

fn do_crash_error() -> anyhow::Result<()> {
    fn inner() -> std::io::Result<()> {
        Err(std::io::ErrorKind::Other.into())
    }

    inner().context("test error")
}

fn do_crash_panic() -> anyhow::Result<()> {
    panic!()
}
