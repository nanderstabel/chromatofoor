use crate::generated::domain::customs_declaration::{
    aggregate::CustomsDeclaration, event::CustomsDeclarationLodged,
};
use chromatofoor::bounded_context::Event;
/// Event: `customs-declaration-lodged` — Emitted when a customs declaration is lodged.
impl Event<CustomsDeclaration> for CustomsDeclarationLodged {
    fn evolve(&self, _state: CustomsDeclaration) -> CustomsDeclaration {
        todo!(
            "implement Event::CustomsDeclaration::evolve for CustomsDeclarationLodged (return _state if stateless)"
        )
    }
}
