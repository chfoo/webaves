use std::time::Duration;

use clap::{ArgMatches, Command};
use webaves::{
    net::{Connect, LocalConnector, NameBuilder},
    service::echo::{EchoRPCClient, SERVICE_NAME},
};

pub fn create_client_command<'h>() -> Command<'h> {
    Command::new("echo").about("Echo service client").hide(true)
}

#[tokio::main]
pub async fn run_client(
    global_matches: &ArgMatches,
    _arg_matches: &ArgMatches,
) -> anyhow::Result<()> {
    let stream = LocalConnector::new(
        NameBuilder::new()
            .current_user()
            .current_dir()
            .name(SERVICE_NAME)
            .build(),
    )
    .connect()
    .await?;
    let transport = webaves::net::rpc::create_transport(stream);
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
