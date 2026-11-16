use async_trait::async_trait;

/// Domain service for filtering profanity from ticket content.
#[chromatofoor::domain_service(scope = crate::generated::domain::Ticketing)]
#[derive(Default, Clone)]
pub struct ProfanityFilter;

#[async_trait]
impl crate::generated::domain::services::profanity_filter::ProfanityFilterService
    for ProfanityFilter
{
    async fn check_content(
        &self,
        _req: crate::generated::domain::services::profanity_filter::CheckContentRequest,
    ) -> Result<
        crate::generated::domain::services::profanity_filter::CheckContentResponse,
        crate::generated::domain::services::profanity_filter::CheckContentError,
    > {
        if _req.content.contains("terrible") || _req.content.contains("hate") {
            Err(
                crate::generated::domain::services::profanity_filter::CheckContentError::Validation(
                    "Profanity detected in the message".to_string(),
                ),
            )
        } else {
            Ok(
                crate::generated::domain::services::profanity_filter::CheckContentResponse {
                    is_clean: true,
                },
            )
        }
    }
}
