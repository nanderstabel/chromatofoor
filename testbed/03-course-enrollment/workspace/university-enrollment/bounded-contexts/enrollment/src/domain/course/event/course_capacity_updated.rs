use crate::generated::domain::course::aggregate::Course;
use crate::generated::domain::course::event::CourseCapacityUpdated;
use chromatofoor::bounded_context::Event;
impl Event<Course> for CourseCapacityUpdated {
    fn evolve(&self, _state: Course) -> Course {
        todo!(
            "implement Event::Course::evolve for CourseCapacityUpdated (return _state if stateless)"
        )
    }
}
