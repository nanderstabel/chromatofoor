use crate::generated::domain::freight_invoice::{
    aggregate::FreightInvoice, command::IssueFreightInvoice, error::FreightInvoiceError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `issue-freight-invoice` — Issues a commercial freight invoice for a booked shipment.
///
/// - Actors: `billing-clerk`
/// - Publishes: `freight-invoice-issued`
impl Command<FreightInvoice> for IssueFreightInvoice {
    type Error = FreightInvoiceError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &FreightInvoice,
    ) -> Result<Vec<FreightInvoiceEvent>, Self::Error> {
        todo!("implement Command::FreightInvoice::decide for IssueFreightInvoice")
    }
}
