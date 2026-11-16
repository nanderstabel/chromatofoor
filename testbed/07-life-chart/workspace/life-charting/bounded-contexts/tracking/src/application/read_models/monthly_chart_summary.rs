use crate::generated::application::read_models::monthly_chart_summary::{
    MonthlyChartSummary, MonthlyChartSummaryEntriesItem,
};
use crate::generated::domain::monthly_chart::event::{
    chart_initialized::ChartInitialized, daily_entry_logged::DailyEntryLogged,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<ChartInitialized> for MonthlyChartSummary {
    async fn project(&self, event: Envelope<ChartInitialized>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.chart_id = Some(event.data.chart_id.to_string());
            rec.year = Some(event.data.year);
            rec.month = Some(event.data.month);
        })?;
        Ok(())
    }
}

impl Project<DailyEntryLogged> for MonthlyChartSummary {
    async fn project(&self, event: Envelope<DailyEntryLogged>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.chart_id = Some(event.data.chart_id.to_string());
            let entry = MonthlyChartSummaryEntriesItem {
                day: Some(event.data.day),
                depression_severity: event.data.depression_severity.clone(),
                dysphoric_mania: Some(event.data.dysphoric_mania),
                mania_severity: event.data.mania_severity.clone(),
                mood_score: Some(event.data.mood_score),
                mood_swings_count: Some(event.data.mood_swings_count),
                other_symptoms: event.data.other_symptoms.clone(),
                sleep_hours: Some(event.data.sleep_hours),
                weight_kg: event.data.weight_kg,
            };
            let entries = rec.entries.get_or_insert_with(Vec::new);
            entries.retain(|item| item.day != Some(event.data.day));
            entries.push(entry);
            entries.sort_by_key(|item| item.day.unwrap_or(0));
        })?;
        Ok(())
    }
}
