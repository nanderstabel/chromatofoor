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
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &Course) -> Result<Vec<CourseEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![CourseEvent::CourseCapacityUpdated(
            CourseCapacityUpdated {
                capacity: self.capacity,
                course_id: self.course_id,
            },
        )])
    }
}
