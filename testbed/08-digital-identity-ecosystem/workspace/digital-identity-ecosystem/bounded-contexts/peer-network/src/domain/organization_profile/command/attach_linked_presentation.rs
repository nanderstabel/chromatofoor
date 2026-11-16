use crate::generated::domain::organization_profile::aggregate::OrganizationProfile;
use crate::generated::domain::organization_profile::command::AttachLinkedPresentation;
use crate::generated::domain::organization_profile::error::OrganizationProfileError;
use crate::generated::domain::organization_profile::event::*;
use chromatofoor::bounded_context::Command;

impl Command<OrganizationProfile> for AttachLinkedPresentation {
    type Error = OrganizationProfileError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &OrganizationProfile,
    ) -> Result<Vec<OrganizationProfileEvent>, Self::Error> {
        Ok(vec![])
    }
}
