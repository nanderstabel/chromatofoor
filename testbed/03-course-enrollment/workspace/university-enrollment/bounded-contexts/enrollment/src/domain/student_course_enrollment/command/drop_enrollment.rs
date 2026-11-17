use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_course_enrollment::command::DropEnrollment;
use crate::generated::domain::student_course_enrollment::error::StudentCourseEnrollmentError;
use crate::generated::domain::student_course_enrollment::event::*;
use chromatofoor::bounded_context::Command;
impl Command<StudentCourseEnrollment> for DropEnrollment {
    type Error = StudentCourseEnrollmentError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &StudentCourseEnrollment,
    ) -> Result<Vec<StudentCourseEnrollmentEvent>, Self::Error> {
        todo!("implement Command::StudentCourseEnrollment::decide for DropEnrollment")
    }
}
