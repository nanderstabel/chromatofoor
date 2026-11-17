use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_course_enrollment::command::DropEnrollment;
use crate::generated::domain::student_course_enrollment::error::StudentCourseEnrollmentError;
use crate::generated::domain::student_course_enrollment::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `drop-enrollment` — Drops a student's active enrollment in a course (idempotent if not currently enrolled).
///
/// - Actors: `student`, `registrar`
/// - Publishes: `enrollment-dropped`
/// - Invariants (`StudentCourseEnrollmentError`):
///   - `course-capacity-exceeded` — Active enrolled students for the target course cannot exceed its current capacity.
///   - `student-load-exceeded` — A student cannot hold more than 5 active course enrollments simultaneously.
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
