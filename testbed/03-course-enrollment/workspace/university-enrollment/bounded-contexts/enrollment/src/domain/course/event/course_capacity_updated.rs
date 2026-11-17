use crate::generated::domain::course::{aggregate::Course, event::CourseCapacityUpdated};
use chromatofoor::bounded_context::Event;
/// Event: `course-capacity-updated` — Emitted when a course offering's seat capacity is updated.
impl Event<Course> for CourseCapacityUpdated {
    fn evolve(&self, _state: Course) -> Course {
        todo!(
            "implement Event::Course::evolve for CourseCapacityUpdated (return _state if stateless)"
        )
    }
}
