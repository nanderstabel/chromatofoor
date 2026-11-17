use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_course_enrollment::event::enrollment_dropped::EnrollmentDropped;
use chromatofoor::bounded_context::Event;
impl Event<StudentCourseEnrollment> for EnrollmentDropped {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for EnrollmentDropped (return _state if stateless)"
        )
    }
}
