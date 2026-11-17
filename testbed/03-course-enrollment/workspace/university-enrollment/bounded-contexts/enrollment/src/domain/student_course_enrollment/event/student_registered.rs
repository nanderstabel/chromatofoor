use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_registration::event::student_registered::StudentRegistered;
use chromatofoor::bounded_context::Event;
/// Event: `student-registered` — Emitted when a student profile is registered.
///
/// Consult Criteria: Student registration event matching the target student-id.
impl Event<StudentCourseEnrollment> for StudentRegistered {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for StudentRegistered (return _state if stateless)"
        )
    }
}
