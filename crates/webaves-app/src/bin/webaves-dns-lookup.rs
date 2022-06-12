#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    webaves_app::dns_lookup::main().await
}
