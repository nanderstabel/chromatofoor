use crate::generated::domain::token_status_list::aggregate::TokenStatusList;
use crate::generated::domain::token_status_list::command::RevokeCredentialIndex;
use crate::generated::domain::token_status_list::error::TokenStatusListError;
use crate::generated::domain::token_status_list::event::*;
use chromatofoor::bounded_context::Command;

impl Command<TokenStatusList> for RevokeCredentialIndex {
    type Error = TokenStatusListError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &TokenStatusList,
    ) -> Result<Vec<TokenStatusListEvent>, Self::Error> {
        let status_list_id = if let TokenStatusList::Created(inner) = aggregate {
            inner
                .status_list_id
                .clone()
                .unwrap_or_else(|| "status-list-2026-v1".to_string())
        } else {
            "status-list-2026-v1".to_string()
        };
        Ok(vec![TokenStatusListEvent::CredentialIndexRevoked(
            CredentialIndexRevoked {
                status_list_id,
                index: self.index,
                reason: self.reason,
            },
        )])
    }
}
