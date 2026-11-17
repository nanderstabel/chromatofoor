use crate::generated::domain::student_course_enrollment::{
    aggregate::StudentCourseEnrollment, command::DropEnrollment,
    error::StudentCourseEnrollmentError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `drop-enrollment` — Drops a student's active enrollment in a course (idempotent if not currently enrolled).
///
/// - Actors: `student`, `registrar`
/// - Publishes: `enrollment-dropped`
/// - Invariants (`StudentCourseEnrollmentError`):
///   - `course-not-published` — Target course must be published before students can enroll in it.
///   - `student-not-registered` — Student must be registered before enrolling in any course.
///   - `course-capacity-exceeded` — Active enrolled students for the target course cannot exceed its current capacity.
///   - `student-load-exceeded` — A student cannot hold more than 5 active course enrollments simultaneously.
impl Command<StudentCourseEnrollment> for DropEnrollment {
    type Error = StudentCourseEnrollmentError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(
        self,
        state: &StudentCourseEnrollment,
    ) -> Result<Vec<StudentCourseEnrollmentEvent>, Self::Error> {
        let key = (self.student_id.to_string(), self.course_id.to_string());
        if !state.active_enrollments.contains(&key) {
            return Ok(vec![]);
        }
        Ok(vec![StudentCourseEnrollmentEvent::EnrollmentDropped(
            EnrollmentDropped {
                course_id: self.course_id,
                student_id: self.student_id,
            },
        )])
    }
}
