use crate::generated::handlers::auto_invoice_on_booking::{
    AutoInvoiceOnBooking, AutoInvoiceOnBookingCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<booking::generated::domain::cargo_booking::event::cargo_booked::CargoBooked>
    for AutoInvoiceOnBooking
{
    fn react(
        &self,
        _event: Envelope<
            booking::generated::domain::cargo_booking::event::cargo_booked::CargoBooked,
        >,
    ) -> Vec<AutoInvoiceOnBookingCommands> {
        todo!("implement React::CargoBooked::react for AutoInvoiceOnBooking")
    }
}
