use crate::generated::domain::VoyageRouting;
use crate::generated::domain::services::route_finder::{
    FindOptimalRouteError, FindOptimalRouteRequest, FindOptimalRouteResponse, RouteFinderService,
};
use async_trait::async_trait;
/// Stateless domain service computing an optimal voyage candidate between two UN/LOCODE ports.
#[chromatofoor::domain_service(scope = VoyageRouting)]
#[derive(Default, Clone)]
pub struct RouteFinder;
#[async_trait]
impl RouteFinderService for RouteFinder {
    /// Function: `find-optimal-route` — Returns the recommended voyage number connecting origin and destination ports.
    async fn find_optimal_route(
        &self,
        _req: FindOptimalRouteRequest,
    ) -> Result<FindOptimalRouteResponse, FindOptimalRouteError> {
        todo!("implement RouteFinderService::find_optimal_route for RouteFinder")
    }
}
