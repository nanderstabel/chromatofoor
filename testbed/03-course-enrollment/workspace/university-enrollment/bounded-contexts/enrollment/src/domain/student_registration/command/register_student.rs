use crate::generated::domain::student_registration::{
    aggregate::StudentRegistration, command::RegisterStudent, error::StudentRegistrationError,
    event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `register-student` — Registers a student in the enrollment bounded context.
///
/// - Actors: `registrar`
/// - Publishes: `student-registered`
impl Command<StudentRegistration> for RegisterStudent {
    type Error = StudentRegistrationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &StudentRegistration,
    ) -> Result<Vec<StudentRegistrationEvent>, Self::Error> {
        Ok(vec![StudentRegistrationEvent::StudentRegistered(
            StudentRegistered {
                full_name: self.full_name,
                student_id: self.student_id,
            },
        )])
    }
}
