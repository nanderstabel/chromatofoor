#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    tracing::info!("Starting HTTP server on 0.0.0.0:3000");
    tracing::info!("Workbench UI available at http://localhost:3000/");
    tracing::info!("GraphiQL UI available at http://localhost:3000/graphiql");
    tracing::info!("MCP endpoint available at http://localhost:3000/mcp");

    bootstrap::Domain::build()
        .await?
        .attach(
            chromatofoor_http::HttpServer::bind("0.0.0.0:3000")
                .attach(chromatofoor_graphql::GraphQL::new())
                .attach(chromatofoor_mcp::Mcp::http()),
        )
        .serve()
        .await
}
