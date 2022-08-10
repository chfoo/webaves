use clap::{ArgMatches, Command};
use serde::{Deserialize, Serialize};
use tarpc::server::Serve;
use tracing::Instrument;
use webaves::{
    net::{rpc::ServiceRunner, LocalListener, NameBuilder},
    service::echo::{EchoRPC, EchoRPCServer},
};

pub fn create_service_command<'h>() -> Command<'h> {
    Command::new("serve").subcommand_required(true).subcommand(
        Command::new("echo-service")
            .about("Echo service")
            .hide(true),
    )
}

#[tokio::main]
pub async fn run(_global_matches: &ArgMatches, arg_matches: &ArgMatches) -> anyhow::Result<()> {
    match arg_matches.subcommand() {
        Some(("echo-service", _sub_matches)) => run_echo().await,
        _ => unreachable!(),
    }
}

fn create_local_listener(name: &str) -> LocalListener {
    LocalListener::new(
        NameBuilder::new()
            .current_user()
            .current_dir()
            .name(name)
            .build(),
    )
}

async fn run_server<S, R>(name: &str, server: S) -> anyhow::Result<()>
where
    S: Serve<R> + Send + Clone + 'static,
    S::Fut: Send,
    R: for<'de> Deserialize<'de> + Send + 'static,
    S::Resp: Serialize + Send + 'static,
{
    let listener = create_local_listener(name);
    let mut runner = ServiceRunner::new(server, listener);

    async move {
        runner.listen()?;
        runner.accept_loop().await?;

        Ok::<(), anyhow::Error>(())
    }
    .await?;

    Ok(())
}

async fn run_echo() -> anyhow::Result<()> {
    run_server(webaves::service::echo::SERVICE_NAME, EchoRPCServer.serve())
        .instrument(tracing::info_span!("echo"))
        .await?;

    Ok(())
}
