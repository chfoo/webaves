use clap::Command;
use tracing::Instrument;
use webaves::service::{
    conn::{Connect, LocalConnector, LocalListener},
    echo::{EchoRPC, EchoRPCClient, EchoRPCServer, SERVICE_NAME},
    rpc::ServerRunner,
};

pub async fn main_server() -> anyhow::Result<()> {
    let command = Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about("Echo service.");
    let command = crate::logging::logging_args(command);
    let arg_matches = command.get_matches();

    crate::logging::set_up_logging(&arg_matches);

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

pub async fn main_client() -> anyhow::Result<()> {
    let command = Command::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about("Echo service client.");
    let command = crate::logging::logging_args(command);
    let arg_matches = command.get_matches();

    crate::logging::set_up_logging(&arg_matches);

    let stream = LocalConnector::new()
        .with_service_id(SERVICE_NAME)
        .connect()
        .await?;
    let transport = webaves::service::rpc::create_transport(stream);
    let client = EchoRPCClient::new(Default::default(), transport).spawn();

    for _ in 0..10 {
        let response = client
            .echo(tarpc::context::current(), "Hello world!".to_string())
            .await?;

        println!("{}", response);
    }

    Ok(())
}
