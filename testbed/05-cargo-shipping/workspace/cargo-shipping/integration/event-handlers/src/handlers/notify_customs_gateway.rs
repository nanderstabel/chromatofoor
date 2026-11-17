use crate::generated::handlers::notify_customs_gateway::NotifyCustomsGateway;
use chromatofoor::{
    architecturekit::{DomainError, Envelope},
    integration::Handle,
};
#[derive(Clone, Default, Debug)]
pub struct NotifyCustomsGatewayDeps {}
impl Handle<
    booking::generated::domain::cargo_booking::event::cargo_assigned_to_route::CargoAssignedToRoute,
> for NotifyCustomsGateway {
    async fn handle(
        &self,
        _event: Envelope<
            booking::generated::domain::cargo_booking::event::cargo_assigned_to_route::CargoAssignedToRoute,
        >,
    ) -> Result<(), DomainError> {
        todo!("implement Handle::CargoAssignedToRoute::handle for NotifyCustomsGateway")
    }
}
