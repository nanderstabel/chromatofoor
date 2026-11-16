use crate::generated::*;

impl ResidencyOnboarding {
    #[must_use]
    pub fn has_ended(&self) -> bool {
        matches!(
            self.status,
            Some(ResidencyOnboardingStatus::Accepted | ResidencyOnboardingStatus::Rejected)
        )
    }
}

impl chromatofoor::integration::OnEvent<LeaseRequested> for ResidencyOnboarding {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<LeaseRequested>,
    ) -> Vec<ResidencyOnboardingCommands> {
        self.apartment_id = Some(event.data.apartment_id);
        self.start_date = Some(event.data.start_date);
        self.end_date = Some(event.data.end_date);
        self.status = Some(ResidencyOnboardingStatus::Verifying);

        vec![
            VerifyApartment {
                household_id: event.data.household_id,
                id: event.data.apartment_id,
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnEvent<Verified> for ResidencyOnboarding {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<Verified>,
    ) -> Vec<ResidencyOnboardingCommands> {
        self.status = Some(ResidencyOnboardingStatus::Accepted);

        vec![
            StartLease {
                apartment_id: self.apartment_id.unwrap_or_default(),
                start_date: self.start_date.unwrap_or_default(),
                end_date: self.end_date.unwrap_or_default(),
                id: event.data.household_id,
            }
            .into(),
        ]
    }
}
