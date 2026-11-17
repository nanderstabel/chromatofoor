use crate::generated::domain::student_course_enrollment::{
    aggregate::StudentCourseEnrollment, event::EnrollmentDropped,
};
use chromatofoor::bounded_context::Event;
/// Event: `enrollment-dropped` — Free-standing bounded-context event emitted when a student drops a course enrollment.
///
/// Consult Criteria: All dropped enrollments matching either the target course-id or target student-id.
impl Event<StudentCourseEnrollment> for EnrollmentDropped {
    fn evolve(&self, mut state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        state
            .active_enrollments
            .remove(&(self.student_id.to_string(), self.course_id.to_string()));
        state
    }
}
