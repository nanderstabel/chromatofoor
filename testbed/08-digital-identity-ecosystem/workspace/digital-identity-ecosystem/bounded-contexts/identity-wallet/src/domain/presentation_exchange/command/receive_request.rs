use crate::generated::domain::presentation_exchange::aggregate::PresentationExchange;
use crate::generated::domain::presentation_exchange::command::ReceiveRequest;
use crate::generated::domain::presentation_exchange::error::PresentationExchangeError;
use crate::generated::domain::presentation_exchange::event::*;
use chromatofoor::bounded_context::Command;

impl Command<PresentationExchange> for ReceiveRequest {
    type Error = PresentationExchangeError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &PresentationExchange,
    ) -> Result<Vec<PresentationExchangeEvent>, Self::Error> {
        Ok(vec![])
    }
}
