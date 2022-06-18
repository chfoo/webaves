use std::time::Duration;

use clap::{ArgMatches, Command};
use tracing::Instrument;
use webaves::service::{
    conn::{Connect, LocalConnector, LocalListener},
    echo::{EchoRPC, EchoRPCClient, EchoRPCServer, SERVICE_NAME},
    rpc::ServerRunner,
};

pub fn create_server_command() -> Command<'static> {
    Command::new("echo-service")
        .about("Echo service.")
        .hide(true)
}

pub async fn run_server(_arg_matches: &ArgMatches) -> anyhow::Result<()> {
    let listener = LocalListener::new().with_service_id(SERVICE_NAME);
    let mut runner = ServerRunner::new(EchoRPCServer.serve(), listener);

    async move {
        runner.listen()?;
        runner.accept_loop().await?;

        Ok::<(), anyhow::Error>(())
    }
    .instrument(tracing::info_span!("echo_service"))
    .await?;

    Ok(())
}

pub fn create_client_command() -> Command<'static> {
    Command::new("echo")
        .about("Echo service client.")
        .hide(true)
}

pub async fn run_client(
    global_matches: &ArgMatches,
    _arg_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let stream = LocalConnector::new()
        .with_service_id(SERVICE_NAME)
        .connect()
        .await?;
    let transport = webaves::service::rpc::create_transport(stream);
    let client = EchoRPCClient::new(Default::default(), transport).spawn();

    let progress_bar = crate::logging::create_and_config_progress_bar(global_matches);
    progress_bar.set_length(10);

    for _ in 0..10 {
        let response = client
            .echo(tarpc::context::current(), "Hello world!".to_string())
            .await?;

        progress_bar.inc(1);
        progress_bar.suspend(|| {
            println!("{}", response);
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    progress_bar.finish_and_clear();

    Ok(())
}
