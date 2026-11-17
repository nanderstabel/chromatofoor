use crate::generated::domain::course::{
    aggregate::{Course, CourseInner},
    event::CoursePublished,
};
use chromatofoor::bounded_context::Event;
/// Event: `course-published` — Emitted when a new course offering is published.
impl Event<Course> for CoursePublished {
    fn evolve(&self, _state: Course) -> Course {
        Course::Created(CourseInner {
            capacity: self.capacity,
            course_id: self.course_id.clone(),
            title: self.title.clone(),
        })
    }
}
