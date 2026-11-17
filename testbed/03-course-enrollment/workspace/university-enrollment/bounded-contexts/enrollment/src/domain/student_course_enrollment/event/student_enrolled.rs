use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use crate::generated::domain::student_course_enrollment::event::student_enrolled::StudentEnrolled;
use chromatofoor::bounded_context::Event;
impl Event<StudentCourseEnrollment> for StudentEnrolled {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for StudentEnrolled (return _state if stateless)"
        )
    }
}
