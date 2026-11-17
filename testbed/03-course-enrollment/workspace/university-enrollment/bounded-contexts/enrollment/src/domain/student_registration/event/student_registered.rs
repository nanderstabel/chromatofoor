use crate::generated::domain::student_registration::{
    aggregate::{StudentRegistration, StudentRegistrationInner},
    event::StudentRegistered,
};
use chromatofoor::bounded_context::Event;
/// Event: `student-registered` — Emitted when a student profile is registered.
impl Event<StudentRegistration> for StudentRegistered {
    fn evolve(&self, _state: StudentRegistration) -> StudentRegistration {
        StudentRegistration::Created(StudentRegistrationInner {
            full_name: self.full_name.clone(),
            student_id: self.student_id.clone(),
        })
    }
}
