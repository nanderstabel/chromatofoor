#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    tracing::info!("Starting GraphQL server on 0.0.0.0:8000");
    tracing::info!("GraphiQL UI available at http://localhost:8000/");

    bootstrap::Domain::build()
        .await?
        .attach(
            chromatofoor_http::HttpServer::bind("0.0.0.0:8000")
                .without_default_routes()
                .attach(chromatofoor_graphql::GraphQL::new()),
        )
        .serve()
        .await
}
