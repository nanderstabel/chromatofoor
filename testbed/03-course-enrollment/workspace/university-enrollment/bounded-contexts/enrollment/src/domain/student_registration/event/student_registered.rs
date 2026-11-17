use crate::generated::domain::student_registration::aggregate::StudentRegistration;
use crate::generated::domain::student_registration::event::StudentRegistered;
use chromatofoor::bounded_context::Event;
impl Event<StudentRegistration> for StudentRegistered {
    fn evolve(&self, _state: StudentRegistration) -> StudentRegistration {
        todo!(
            "implement Event::StudentRegistration::evolve for StudentRegistered (return _state if stateless)"
        )
    }
}
