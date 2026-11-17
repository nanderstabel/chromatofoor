use crate::generated::domain::customs_declaration::{
    aggregate::{CustomsDeclaration, CustomsDeclarationInner},
    event::CustomsDeclarationLodged,
};
use chromatofoor::bounded_context::Event;
/// Event: `customs-declaration-lodged` — Emitted when a customs declaration is lodged.
impl Event<CustomsDeclaration> for CustomsDeclarationLodged {
    fn evolve(&self, _state: CustomsDeclaration) -> CustomsDeclaration {
        CustomsDeclaration::Created(CustomsDeclarationInner {
            declaration_id: self.declaration_id.clone(),
            tracking_id: self.tracking_id.clone(),
        })
    }
}
