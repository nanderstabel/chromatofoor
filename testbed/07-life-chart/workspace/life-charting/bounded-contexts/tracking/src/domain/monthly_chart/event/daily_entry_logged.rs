use crate::generated::domain::monthly_chart::aggregate::MonthlyChart;
use crate::generated::domain::monthly_chart::event::DailyEntryLogged;
use chromatofoor::bounded_context::Event;

impl Event<MonthlyChart> for DailyEntryLogged {
    fn evolve(&self, state: MonthlyChart) -> MonthlyChart {
        // TODO: Implement event evolution logic
        state
    }
}
