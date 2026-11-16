#[allow(unused_imports)]
use crate::generated::application::read_models::template_catalog::{
    TemplateCatalog, TemplateCatalogRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<crate::generated::domain::credential_template::event::template_created::TemplateCreated>
    for TemplateCatalog
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::credential_template::event::template_created::TemplateCreated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
