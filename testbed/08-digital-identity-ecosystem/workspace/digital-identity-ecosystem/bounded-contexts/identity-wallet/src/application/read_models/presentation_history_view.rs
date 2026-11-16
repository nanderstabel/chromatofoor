#[allow(unused_imports)]
use crate::generated::application::read_models::presentation_history_view::{
    PresentationHistoryView, PresentationHistoryViewRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<
        crate::generated::domain::presentation_exchange::event::request_received::RequestReceived,
    > for PresentationHistoryView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::presentation_exchange::event::request_received::RequestReceived,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::presentation_exchange::event::presentation_authorized::PresentationAuthorized,
> for PresentationHistoryView {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::presentation_exchange::event::presentation_authorized::PresentationAuthorized,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
