use crate::generated::domain::course::aggregate::Course;
use crate::generated::domain::course::event::CoursePublished;
use chromatofoor::bounded_context::Event;
/// Event: `course-published` — Emitted when a new course offering is published.
impl Event<Course> for CoursePublished {
    fn evolve(&self, _state: Course) -> Course {
        todo!("implement Event::Course::evolve for CoursePublished (return _state if stateless)")
    }
}
