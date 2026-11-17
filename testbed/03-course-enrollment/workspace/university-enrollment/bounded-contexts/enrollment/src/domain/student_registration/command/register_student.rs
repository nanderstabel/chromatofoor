use crate::generated::domain::student_registration::aggregate::StudentRegistration;
use crate::generated::domain::student_registration::command::RegisterStudent;
use crate::generated::domain::student_registration::error::StudentRegistrationError;
use crate::generated::domain::student_registration::event::*;
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
        todo!("implement Command::StudentRegistration::decide for RegisterStudent")
    }
}
