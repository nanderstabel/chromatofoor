use crate::generated::domain::customs_declaration::aggregate::CustomsDeclaration;
use crate::generated::domain::customs_declaration::command::LodgeCustomsDeclaration;
use crate::generated::domain::customs_declaration::error::CustomsDeclarationError;
use crate::generated::domain::customs_declaration::event::*;
use chromatofoor::bounded_context::Command;
impl Command<CustomsDeclaration> for LodgeCustomsDeclaration {
    type Error = CustomsDeclarationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &CustomsDeclaration,
    ) -> Result<Vec<CustomsDeclarationEvent>, Self::Error> {
        todo!("implement Command::CustomsDeclaration::decide for LodgeCustomsDeclaration")
    }
}
