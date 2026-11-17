use crate::generated::handlers::auto_invoice_on_booking::{
    AutoInvoiceOnBooking, AutoInvoiceOnBookingCommands, CargoBooked,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
/// Policy: `auto-invoice-on-booking` — Stateless reactive policy that issues a freight invoice whenever new cargo is booked.
/// Trigger: `booking` / `cargo-booking` / `cargo-booked`
///
/// - Emits: `billing/freight-invoice/issue-freight-invoice`
impl React<CargoBooked> for AutoInvoiceOnBooking {
    fn react(&self, _event: Envelope<CargoBooked>) -> Vec<AutoInvoiceOnBookingCommands> {
        todo!("implement React::CargoBooked::react for AutoInvoiceOnBooking")
    }
}
