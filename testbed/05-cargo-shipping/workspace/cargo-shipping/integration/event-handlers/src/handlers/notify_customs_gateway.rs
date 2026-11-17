use crate::generated::handlers::notify_customs_gateway::{
    CargoAssignedToRoute, NotifyCustomsGateway,
};
use chromatofoor::{
    architecturekit::{DomainError, Envelope},
    integration::Handle,
};
#[derive(Clone, Default, Debug)]
pub struct NotifyCustomsGatewayDeps {}
/// Event Handler: `notify-customs-gateway` — Outbound event handler submitting an electronic manifest to the external customs gateway when cargo is routed.
/// Trigger: `booking` / `cargo-booking` / `cargo-assigned-to-route`
///
/// - Side Effects:
///   - `external-call` (`customs-clearance-gateway`) — Submit electronic cargo manifest to customs-clearance-gateway for the assigned voyage.
impl Handle<CargoAssignedToRoute> for NotifyCustomsGateway {
    async fn handle(&self, event: Envelope<CargoAssignedToRoute>) -> Result<(), DomainError> {
        chromatofoor::tracing::info!(
            tracking_id = %event.data.tracking_id,
            voyage_number = %event.data.voyage_number,
            "submitting electronic cargo manifest to customs-clearance-gateway"
        );
        Ok(())
    }
}
