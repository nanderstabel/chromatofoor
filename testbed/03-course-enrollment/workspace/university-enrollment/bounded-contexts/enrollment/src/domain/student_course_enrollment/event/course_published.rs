use crate::generated::domain::course::event::course_published::CoursePublished;
use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use chromatofoor::bounded_context::Event;
impl Event<StudentCourseEnrollment> for CoursePublished {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for CoursePublished (return _state if stateless)"
        )
    }
}
