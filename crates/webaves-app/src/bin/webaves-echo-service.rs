#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    webaves_app::echo::main_server().await
}
