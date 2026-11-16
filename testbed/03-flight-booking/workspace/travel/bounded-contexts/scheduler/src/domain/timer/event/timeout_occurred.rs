use crate::generated::domain::timer::aggregate::Timer;
use crate::generated::domain::timer::event::TimeoutOccurred;
use chromatofoor::bounded_context::Event;

impl Event<Timer> for TimeoutOccurred {
    fn evolve(&self, state: Timer) -> Timer {
        // TODO: Implement event evolution logic
        state
    }
}
