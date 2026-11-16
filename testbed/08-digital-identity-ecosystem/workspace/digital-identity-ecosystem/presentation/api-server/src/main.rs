#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    tracing::info!("Starting HTTP server on 0.0.0.0:3000");

    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_http::HttpServer::bind("0.0.0.0:3000"))
        .serve()
        .await
}
