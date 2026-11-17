use crate::generated::domain::student_registration::{
    aggregate::StudentRegistration, event::StudentRegistered,
};
use chromatofoor::bounded_context::Event;
/// Event: `student-registered` — Emitted when a student profile is registered.
impl Event<StudentRegistration> for StudentRegistered {
    fn evolve(&self, _state: StudentRegistration) -> StudentRegistration {
        todo!(
            "implement Event::StudentRegistration::evolve for StudentRegistered (return _state if stateless)"
        )
    }
}
