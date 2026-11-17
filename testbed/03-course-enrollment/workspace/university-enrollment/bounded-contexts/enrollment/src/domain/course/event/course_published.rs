use crate::generated::domain::course::aggregate::Course;
use crate::generated::domain::course::event::CoursePublished;
use chromatofoor::bounded_context::Event;
impl Event<Course> for CoursePublished {
    fn evolve(&self, _state: Course) -> Course {
        todo!("implement Event::Course::evolve for CoursePublished (return _state if stateless)")
    }
}
