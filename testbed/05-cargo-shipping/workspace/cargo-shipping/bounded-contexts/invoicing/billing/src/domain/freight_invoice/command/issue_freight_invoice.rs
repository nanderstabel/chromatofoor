use crate::generated::domain::freight_invoice::aggregate::FreightInvoice;
use crate::generated::domain::freight_invoice::command::IssueFreightInvoice;
use crate::generated::domain::freight_invoice::error::FreightInvoiceError;
use crate::generated::domain::freight_invoice::event::*;
use chromatofoor::bounded_context::Command;
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
