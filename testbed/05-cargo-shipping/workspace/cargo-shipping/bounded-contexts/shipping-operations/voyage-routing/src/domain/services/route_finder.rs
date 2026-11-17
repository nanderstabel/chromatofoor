use async_trait::async_trait;
/// Stateless domain service computing an optimal voyage candidate between two UN/LOCODE ports.
#[chromatofoor::domain_service(scope = crate::generated::domain::VoyageRouting)]
#[derive(Default, Clone)]
pub struct RouteFinder;
#[async_trait]
impl crate::generated::domain::services::route_finder::RouteFinderService for RouteFinder {
    /// Function: `find-optimal-route` — Returns the recommended voyage number connecting origin and destination ports.
    async fn find_optimal_route(
        &self,
        _req: crate::generated::domain::services::route_finder::FindOptimalRouteRequest,
    ) -> Result<
        crate::generated::domain::services::route_finder::FindOptimalRouteResponse,
        crate::generated::domain::services::route_finder::FindOptimalRouteError,
    > {
        todo!("implement RouteFinderService::find_optimal_route for RouteFinder")
    }
}
