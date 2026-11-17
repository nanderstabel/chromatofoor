use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_course_enrollment::event::enrollment_dropped::EnrollmentDropped;
use chromatofoor::bounded_context::Event;
/// Event: `enrollment-dropped` — Free-standing bounded-context event emitted when a student drops a course enrollment.
///
/// Consult Criteria: All dropped enrollments matching either the target course-id or target student-id.
impl Event<StudentCourseEnrollment> for EnrollmentDropped {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for EnrollmentDropped (return _state if stateless)"
        )
    }
}
