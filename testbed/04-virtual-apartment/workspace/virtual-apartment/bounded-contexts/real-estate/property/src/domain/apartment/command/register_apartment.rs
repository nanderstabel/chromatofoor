use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::command::RegisterApartment;
use crate::generated::domain::apartment::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Apartment> for RegisterApartment {
    type Error = crate::generated::domain::apartment::error::ApartmentError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Apartment) -> Result<Vec<ApartmentEvent>, Self::Error> {
        use crate::generated::domain::apartment::error::ApartmentError;
        use crate::generated::domain::apartment::event::ApartmentEvent;
        use crate::generated::domain::apartment::event::registered::Registered;

        if !_aggregate.is_none() {
            return Err(ApartmentError::Validation("Apartment already registered".into()).into());
        }

        Ok(vec![ApartmentEvent::Registered(Registered {
            address: self.address,
            door_bell: self.door_bell,
        })])
    }
}
