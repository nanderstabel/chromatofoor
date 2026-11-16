use crate::generated::domain::monthly_chart::aggregate::MonthlyChart;
use crate::generated::domain::monthly_chart::command::InitializeChart;
use crate::generated::domain::monthly_chart::error::MonthlyChartError;
use crate::generated::domain::monthly_chart::event::*;
use chromatofoor::bounded_context::Command;

impl Command<MonthlyChart> for InitializeChart {
    type Error = MonthlyChartError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &MonthlyChart,
    ) -> Result<Vec<MonthlyChartEvent>, Self::Error> {
        use crate::generated::domain::monthly_chart::event::{ChartInitialized, MonthlyChartEvent};
        Ok(vec![MonthlyChartEvent::ChartInitialized(
            ChartInitialized {
                chart_id: self.chart_id,
                month: self.month,
                year: self.year,
            },
        )])
    }
}
