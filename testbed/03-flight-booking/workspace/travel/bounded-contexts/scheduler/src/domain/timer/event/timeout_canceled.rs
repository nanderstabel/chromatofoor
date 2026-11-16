use crate::generated::domain::timer::aggregate::Timer;
use crate::generated::domain::timer::event::TimeoutCanceled;
use chromatofoor::bounded_context::Event;

impl Event<Timer> for TimeoutCanceled {
    fn evolve(&self, state: Timer) -> Timer {
        // TODO: Implement event evolution logic
        state
    }
}
