#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    tracing::info!("Starting MCP server over stdio");

    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_mcp::Mcp::stdio())
        .serve()
        .await
}
