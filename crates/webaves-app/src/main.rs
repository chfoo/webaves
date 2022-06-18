mod argutil;
mod dns_lookup;
mod echo;
mod logging;
mod warc;

use clap::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command = Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .subcommand_required(true)
        .subcommand(crate::dns_lookup::create_command())
        .subcommand(crate::echo::create_server_command())
        .subcommand(crate::echo::create_client_command())
        .subcommand(crate::warc::create_command());

    let command = crate::logging::logging_args(command);

    let arg_matches = command.get_matches();

    crate::logging::set_up_logging(&arg_matches)?;

    let result = match arg_matches.subcommand() {
        Some(("dns-lookup", sub_matches)) => crate::dns_lookup::run(sub_matches).await,
        Some(("echo-service", sub_matches)) => crate::echo::run_server(sub_matches).await,
        Some(("echo", sub_matches)) => crate::echo::run_client(&arg_matches, sub_matches).await,
        Some(("warc", sub_matches)) => crate::warc::run(&arg_matches, sub_matches),
        _ => unreachable!(),
    };

    match result {
        Ok(_) => {
            tracing::info!("program exit ok");
            Ok(())
        }
        Err(error) => {
            tracing::error!(%error, "program exit error");
            Err(error)
        },
    }
}
