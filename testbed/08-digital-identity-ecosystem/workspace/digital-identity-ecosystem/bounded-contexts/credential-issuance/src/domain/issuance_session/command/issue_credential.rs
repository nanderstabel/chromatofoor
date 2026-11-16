use crate::generated::domain::issuance_session::aggregate::IssuanceSession;
use crate::generated::domain::issuance_session::command::IssueCredential;
use crate::generated::domain::issuance_session::error::IssuanceSessionError;
use crate::generated::domain::issuance_session::event::*;
use chromatofoor::bounded_context::Command;

impl Command<IssuanceSession> for IssueCredential {
    type Error = IssuanceSessionError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &IssuanceSession,
    ) -> Result<Vec<IssuanceSessionEvent>, Self::Error> {
        let session_id = if let IssuanceSession::Created(inner) = aggregate {
            inner
                .session_id
                .clone()
                .unwrap_or_else(|| "session-oid4vci-999".to_string())
        } else {
            "session-oid4vci-999".to_string()
        };
        Ok(vec![IssuanceSessionEvent::CredentialIssued(
            CredentialIssued {
                session_id,
                credential_payload: self.credential_payload,
                status_list_index: self.status_list_index,
            },
        )])
    }
}
