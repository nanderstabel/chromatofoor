use crate::generated::application::read_models::active_timers::{
    ActiveTimers, ActiveTimersTimersItem,
};
use crate::generated::domain::timer::event::{
    timeout_canceled::TimeoutCanceled, timeout_occurred::TimeoutOccurred,
    timeout_scheduled::TimeoutScheduled,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::{DocumentView, Project},
};

impl Project<TimeoutScheduled> for ActiveTimers {
    async fn project(&self, event: Envelope<TimeoutScheduled>) -> Result<(), ViewError> {
        self.records.upsert("singleton", &event, |rec| {
            let item = ActiveTimersTimersItem {
                timer_id: Some(event.data.timer_id.to_string()),
                process_manager_id: Some(event.data.process_manager_id.clone()),
                timer_name: Some(event.data.timer_name.clone()),
                fire_at: Some(event.data.fire_at),
            };
            rec.timers.get_or_insert_with(Vec::new).push(item);
        })?;
        Ok(())
    }
}

impl Project<TimeoutCanceled> for ActiveTimers {
    async fn project(&self, event: Envelope<TimeoutCanceled>) -> Result<(), ViewError> {
        let timer_id = DocumentView::<ActiveTimersTimersItem>::subject_id(&event).to_owned();
        self.records.upsert("singleton", &event, |rec| {
            if let Some(timers) = &mut rec.timers {
                timers.retain(|t| t.timer_id.as_deref() != Some(timer_id.as_str()));
            }
        })?;
        Ok(())
    }
}

impl Project<TimeoutOccurred> for ActiveTimers {
    async fn project(&self, event: Envelope<TimeoutOccurred>) -> Result<(), ViewError> {
        let timer_id = DocumentView::<ActiveTimersTimersItem>::subject_id(&event).to_owned();
        self.records.upsert("singleton", &event, |rec| {
            if let Some(timers) = &mut rec.timers {
                timers.retain(|t| t.timer_id.as_deref() != Some(timer_id.as_str()));
            }
        })?;
        Ok(())
    }
}
