#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_tui::Tui)
        .serve()
        .await
}
