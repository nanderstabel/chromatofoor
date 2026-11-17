use crate::generated::domain::{
    course::event::CourseCapacityUpdated,
    student_course_enrollment::aggregate::StudentCourseEnrollment,
};
use chromatofoor::bounded_context::Event;
/// Event: `course-capacity-updated` — Emitted when a course offering's seat capacity is updated.
///
/// Consult Criteria: Capacity updates matching the target course-id.
impl Event<StudentCourseEnrollment> for CourseCapacityUpdated {
    fn evolve(&self, mut state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        state
            .course_capacities
            .insert(self.course_id.to_string(), self.capacity);
        state
    }
}
