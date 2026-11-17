use crate::generated::domain::course::{aggregate::Course, event::CourseCapacityUpdated};
use chromatofoor::bounded_context::Event;
/// Event: `course-capacity-updated` — Emitted when a course offering's seat capacity is updated.
impl Event<Course> for CourseCapacityUpdated {
    fn evolve(&self, mut state: Course) -> Course {
        if let Course::Created(ref mut inner) = state {
            inner.capacity = self.capacity;
        }
        state
    }
}
