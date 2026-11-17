use crate::generated::domain::customs_declaration::{
    aggregate::CustomsDeclaration, command::LodgeCustomsDeclaration,
    error::CustomsDeclarationError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `lodge-customs-declaration` — Lodges a customs declaration for a booked cargo shipment.
///
/// - Actors: `customs-officer`
/// - Publishes: `customs-declaration-lodged`
impl Command<CustomsDeclaration> for LodgeCustomsDeclaration {
    type Error = CustomsDeclarationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &CustomsDeclaration,
    ) -> Result<Vec<CustomsDeclarationEvent>, Self::Error> {
        Ok(vec![CustomsDeclarationEvent::CustomsDeclarationLodged(
            CustomsDeclarationLodged {
                declaration_id: self.declaration_id,
                tracking_id: self.tracking_id,
            },
        )])
    }
}
