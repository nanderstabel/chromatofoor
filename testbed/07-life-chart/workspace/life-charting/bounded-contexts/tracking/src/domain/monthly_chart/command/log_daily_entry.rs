use crate::generated::domain::monthly_chart::aggregate::MonthlyChart;
use crate::generated::domain::monthly_chart::command::LogDailyEntry;
use crate::generated::domain::monthly_chart::error::MonthlyChartError;
use crate::generated::domain::monthly_chart::event::*;
use chromatofoor::bounded_context::Command;

impl Command<MonthlyChart> for LogDailyEntry {
    type Error = MonthlyChartError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &MonthlyChart,
    ) -> Result<Vec<MonthlyChartEvent>, Self::Error> {
        use crate::generated::domain::monthly_chart::event::{DailyEntryLogged, MonthlyChartEvent};
        Ok(vec![MonthlyChartEvent::DailyEntryLogged(
            DailyEntryLogged {
                chart_id: self.chart_id,
                day: self.day,
                sleep_hours: self.sleep_hours,
                weight_kg: self.weight_kg,
                dysphoric_mania: self.dysphoric_mania,
                mania_severity: self.mania_severity,
                depression_severity: self.depression_severity,
                mood_score: self.mood_score,
                mood_swings_count: self.mood_swings_count,
                other_symptoms: self.other_symptoms,
            },
        )])
    }
}
