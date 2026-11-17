use crate::generated::domain::{
    student_course_enrollment::aggregate::StudentCourseEnrollment,
    student_registration::event::StudentRegistered,
};
use chromatofoor::bounded_context::Event;
/// Event: `student-registered` — Emitted when a student profile is registered.
///
/// Consult Criteria: Student registration event matching the target student-id.
impl Event<StudentCourseEnrollment> for StudentRegistered {
    fn evolve(&self, mut state: StudentCourseEnrollment) -> StudentCourseEnrollment {
        state
            .registered_students
            .insert(self.student_id.to_string());
        state
    }
}
