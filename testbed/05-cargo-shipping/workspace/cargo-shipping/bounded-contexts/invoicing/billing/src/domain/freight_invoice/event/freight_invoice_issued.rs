use crate::generated::domain::freight_invoice::aggregate::FreightInvoice;
use crate::generated::domain::freight_invoice::event::FreightInvoiceIssued;
use chromatofoor::bounded_context::Event;
impl Event<FreightInvoice> for FreightInvoiceIssued {
    fn evolve(&self, _state: FreightInvoice) -> FreightInvoice {
        todo!(
            "implement Event::FreightInvoice::evolve for FreightInvoiceIssued (return _state if stateless)"
        )
    }
}
