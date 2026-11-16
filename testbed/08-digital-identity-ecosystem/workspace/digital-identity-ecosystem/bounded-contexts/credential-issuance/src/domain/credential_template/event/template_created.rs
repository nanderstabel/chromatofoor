use crate::generated::domain::credential_template::aggregate::CredentialTemplate;
use crate::generated::domain::credential_template::event::TemplateCreated;
use chromatofoor::bounded_context::Event;

impl Event<CredentialTemplate> for TemplateCreated {
    fn evolve(&self, state: CredentialTemplate) -> CredentialTemplate {
        // TODO: Implement event evolution logic
        state
    }
}
