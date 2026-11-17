use crate::generated::domain::student_course_enrollment::{
    aggregate::StudentCourseEnrollment, event::StudentEnrolled,
};
use chromatofoor::bounded_context::Event;
/// Event: `student-enrolled` — Free-standing bounded-context event emitted when a student enrolls in a course.
///
/// Consult Criteria: All enrollments matching either the target course-id (for seat count) or target student-id (for course load).
impl Event<StudentCourseEnrollment> for StudentEnrolled {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for StudentEnrolled (return _state if stateless)"
        )
    }
}
