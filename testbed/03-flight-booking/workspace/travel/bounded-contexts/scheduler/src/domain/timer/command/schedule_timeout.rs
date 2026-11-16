use crate::generated::domain::timer::aggregate::Timer;
use crate::generated::domain::timer::command::ScheduleTimeout;
use crate::generated::domain::timer::error::TimerError;
use crate::generated::domain::timer::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Timer> for ScheduleTimeout {
    type Error = TimerError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Timer) -> Result<Vec<TimerEvent>, Self::Error> {
        Ok(vec![TimerEvent::TimeoutScheduled(TimeoutScheduled {
            fire_at: self.fire_at,
            process_manager_id: self.process_manager_id.clone(),
            timer_id: self.timer_id.clone(),
            timer_name: self.timer_name.clone(),
        })])
    }
}
