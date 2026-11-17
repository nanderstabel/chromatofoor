use crate::generated::handlers::auto_invoice_on_booking::{
    AutoInvoiceOnBooking, AutoInvoiceOnBookingCommands, CargoBooked, IssueFreightInvoice,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
/// Policy: `auto-invoice-on-booking` — Stateless reactive policy that issues a freight invoice whenever new cargo is booked.
/// Trigger: `booking` / `cargo-booking` / `cargo-booked`
///
/// - Emits: `billing/freight-invoice/issue-freight-invoice`
impl React<CargoBooked> for AutoInvoiceOnBooking {
    fn react(&self, event: Envelope<CargoBooked>) -> Vec<AutoInvoiceOnBookingCommands> {
        let tracking_id = event.data.tracking_id;
        let invoice_id: chromatofoor::Identifier = format!("inv-{tracking_id}").into();
        vec![
            IssueFreightInvoice {
                amount_cents: 100_000,
                invoice_id,
                tracking_id,
            }
            .into(),
        ]
    }
}
