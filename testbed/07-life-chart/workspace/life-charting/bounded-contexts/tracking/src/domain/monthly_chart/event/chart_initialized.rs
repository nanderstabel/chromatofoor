use crate::generated::domain::monthly_chart::aggregate::MonthlyChart;
use crate::generated::domain::monthly_chart::event::ChartInitialized;
use chromatofoor::bounded_context::Event;

impl Event<MonthlyChart> for ChartInitialized {
    fn evolve(&self, state: MonthlyChart) -> MonthlyChart {
        // TODO: Implement event evolution logic
        state
    }
}
