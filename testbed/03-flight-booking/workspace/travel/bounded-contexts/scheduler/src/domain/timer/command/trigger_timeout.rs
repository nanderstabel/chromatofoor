use crate::generated::domain::timer::aggregate::Timer;
use crate::generated::domain::timer::command::TriggerTimeout;
use crate::generated::domain::timer::error::TimerError;
use crate::generated::domain::timer::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Timer> for TriggerTimeout {
    type Error = TimerError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Timer) -> Result<Vec<TimerEvent>, Self::Error> {
        Ok(vec![TimerEvent::TimeoutOccurred(TimeoutOccurred {
            process_manager_id: self.process_manager_id.clone(),
            timer_name: self.timer_name.clone(),
        })])
    }
}
