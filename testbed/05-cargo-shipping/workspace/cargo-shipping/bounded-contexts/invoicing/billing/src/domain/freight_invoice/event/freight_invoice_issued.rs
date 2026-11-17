use crate::generated::domain::freight_invoice::aggregate::FreightInvoice;
use crate::generated::domain::freight_invoice::event::FreightInvoiceIssued;
use chromatofoor::bounded_context::Event;
/// Event: `freight-invoice-issued` — Emitted when a freight invoice is issued.
impl Event<FreightInvoice> for FreightInvoiceIssued {
    fn evolve(&self, _state: FreightInvoice) -> FreightInvoice {
        todo!(
            "implement Event::FreightInvoice::evolve for FreightInvoiceIssued (return _state if stateless)"
        )
    }
}
