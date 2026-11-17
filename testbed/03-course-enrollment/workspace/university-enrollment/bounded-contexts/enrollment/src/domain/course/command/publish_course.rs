use crate::generated::domain::course::aggregate::Course;
use crate::generated::domain::course::command::PublishCourse;
use crate::generated::domain::course::error::CourseError;
use crate::generated::domain::course::event::*;
use chromatofoor::bounded_context::Command;
impl Command<Course> for PublishCourse {
    type Error = CourseError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Course) -> Result<Vec<CourseEvent>, Self::Error> {
        todo!("implement Command::Course::decide for PublishCourse")
    }
}
