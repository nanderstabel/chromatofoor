use crate::generated::domain::course::{
    aggregate::Course, command::PublishCourse, error::CourseError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `publish-course` — Publishes a new course offering with an initial seat capacity.
///
/// - Actors: `registrar`
/// - Publishes: `course-published`
impl Command<Course> for PublishCourse {
    type Error = CourseError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Course) -> Result<Vec<CourseEvent>, Self::Error> {
        Ok(vec![CourseEvent::CoursePublished(CoursePublished {
            capacity: self.capacity,
            course_id: self.course_id,
            title: self.title,
        })])
    }
}
