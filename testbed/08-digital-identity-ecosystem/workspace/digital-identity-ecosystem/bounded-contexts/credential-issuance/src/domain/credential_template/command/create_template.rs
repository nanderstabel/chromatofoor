use crate::generated::domain::credential_template::aggregate::CredentialTemplate;
use crate::generated::domain::credential_template::command::CreateTemplate;
use crate::generated::domain::credential_template::error::CredentialTemplateError;
use crate::generated::domain::credential_template::event::*;
use chromatofoor::bounded_context::Command;

impl Command<CredentialTemplate> for CreateTemplate {
    type Error = CredentialTemplateError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &CredentialTemplate,
    ) -> Result<Vec<CredentialTemplateEvent>, Self::Error> {
        Ok(vec![])
    }
}
