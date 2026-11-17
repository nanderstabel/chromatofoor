use crate::generated::domain::freight_invoice::{
    aggregate::{FreightInvoice, FreightInvoiceInner},
    event::FreightInvoiceIssued,
};
use chromatofoor::bounded_context::Event;
/// Event: `freight-invoice-issued` — Emitted when a freight invoice is issued.
impl Event<FreightInvoice> for FreightInvoiceIssued {
    fn evolve(&self, _state: FreightInvoice) -> FreightInvoice {
        FreightInvoice::Created(FreightInvoiceInner {
            amount_cents: self.amount_cents,
            invoice_id: self.invoice_id.clone(),
            tracking_id: self.tracking_id.clone(),
        })
    }
}
