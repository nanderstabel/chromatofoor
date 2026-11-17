use crate::generated::domain::course::event::course_published::CoursePublished;
use crate::generated::domain::student_course_enrollment::aggregate::StudentCourseEnrollment;
use chromatofoor::bounded_context::Event;
/// Event: `course-published` — Emitted when a new course offering is published.
///
/// Consult Criteria: Course publication event matching the target course-id.
impl Event<StudentCourseEnrollment> for CoursePublished {
    fn evolve(&self, _state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        todo!(
            "implement Event::StudentCourseEnrollment::evolve for CoursePublished (return _state if stateless)"
        )
    }
}
