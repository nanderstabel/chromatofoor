use crate::generated::domain::organization_profile::aggregate::OrganizationProfile;
use crate::generated::domain::organization_profile::command::UpdateProfile;
use crate::generated::domain::organization_profile::error::OrganizationProfileError;
use crate::generated::domain::organization_profile::event::*;
use chromatofoor::bounded_context::Command;

impl Command<OrganizationProfile> for UpdateProfile {
    type Error = OrganizationProfileError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &OrganizationProfile,
    ) -> Result<Vec<OrganizationProfileEvent>, Self::Error> {
        Ok(vec![])
    }
}
