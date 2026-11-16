use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::command::VerifyApartment;
use crate::generated::domain::apartment::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Apartment> for VerifyApartment {
    type Error = crate::generated::domain::apartment::error::ApartmentError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Apartment) -> Result<Vec<ApartmentEvent>, Self::Error> {
        use crate::generated::domain::apartment::error::ApartmentError;
        use crate::generated::domain::apartment::event::ApartmentEvent;
        use crate::generated::domain::apartment::event::verified::Verified;

        if _aggregate.is_none() {
            return Err(ApartmentError::Validation("Apartment not registered".into()).into());
        }

        Ok(vec![ApartmentEvent::Verified(Verified {
            household_id: self.household_id,
        })])
    }
}
