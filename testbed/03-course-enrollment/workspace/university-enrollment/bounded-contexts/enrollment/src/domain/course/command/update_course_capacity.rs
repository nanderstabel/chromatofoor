use crate::generated::domain::course::{
    aggregate::Course, command::UpdateCourseCapacity, error::CourseError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `update-course-capacity` — Updates the maximum seat capacity of an existing course.
///
/// - Actors: `registrar`
/// - Publishes: `course-capacity-updated`
impl Command<Course> for UpdateCourseCapacity {
    type Error = CourseError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Course) -> Result<Vec<CourseEvent>, Self::Error> {
        todo!("implement Command::Course::decide for UpdateCourseCapacity")
    }
}
