use crate::generated::domain::customs_declaration::aggregate::CustomsDeclaration;
use crate::generated::domain::customs_declaration::event::CustomsDeclarationLodged;
use chromatofoor::bounded_context::Event;
impl Event<CustomsDeclaration> for CustomsDeclarationLodged {
    fn evolve(&self, _state: CustomsDeclaration) -> CustomsDeclaration {
        todo!(
            "implement Event::CustomsDeclaration::evolve for CustomsDeclarationLodged (return _state if stateless)"
        )
    }
}
