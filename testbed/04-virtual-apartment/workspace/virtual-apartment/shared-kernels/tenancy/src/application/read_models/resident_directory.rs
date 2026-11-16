#[allow(unused_imports)]
use crate::generated::application::read_models::resident_directory::{
    ResidentDirectory, ResidentDirectoryRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::household::event::formed::Formed> for ResidentDirectory {
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::household::event::formed::Formed>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::resident_added::ResidentAdded>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::household::event::resident_added::ResidentAdded>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::resident_removed::ResidentRemoved>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::household::event::resident_removed::ResidentRemoved,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::lease_started::LeaseStarted>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::household::event::lease_started::LeaseStarted>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::lease_ended::LeaseEnded>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::household::event::lease_ended::LeaseEnded>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::lease_requested::LeaseRequested>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::household::event::lease_requested::LeaseRequested,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::household::event::lease_rejected::LeaseRejected>
    for ResidentDirectory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::household::event::lease_rejected::LeaseRejected>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
