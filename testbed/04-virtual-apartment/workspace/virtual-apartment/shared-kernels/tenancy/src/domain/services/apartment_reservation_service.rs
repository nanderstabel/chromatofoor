use async_trait::async_trait;
/// Domain service for managing apartment reservations.
#[chromatofoor::domain_service(scope = crate::generated::domain::Tenancy)]
#[derive(Default, Clone)]
pub struct ApartmentReservationService;
#[async_trait]
impl crate::generated::domain::services::apartment_reservation_service::ApartmentReservationServiceService
for ApartmentReservationService {
    async fn apartment_reservation_logic(
        &self,
        _req: crate::generated::domain::services::apartment_reservation_service::ApartmentReservationLogicRequest,
    ) -> Result<
        crate::generated::domain::services::apartment_reservation_service::ApartmentReservationLogicResponse,
        crate::generated::domain::services::apartment_reservation_service::ApartmentReservationLogicError,
    > {
        todo!("Implement apartment_reservation_logic for ApartmentReservationService")
    }
}
