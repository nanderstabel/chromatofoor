use crate::generated::domain::{
    course::event::CoursePublished, student_course_enrollment::aggregate::StudentCourseEnrollment,
};
use chromatofoor::bounded_context::Event;
/// Event: `course-published` — Emitted when a new course offering is published.
///
/// Consult Criteria: Course publication event matching the target course-id.
impl Event<StudentCourseEnrollment> for CoursePublished {
    fn evolve(&self, mut state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        state
            .course_capacities
            .insert(self.course_id.to_string(), self.capacity);
        state
    }
}
