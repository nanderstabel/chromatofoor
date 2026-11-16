use crate::generated::domain::token_status_list::aggregate::TokenStatusList;
use crate::generated::domain::token_status_list::command::CreateStatusList;
use crate::generated::domain::token_status_list::error::TokenStatusListError;
use crate::generated::domain::token_status_list::event::*;
use chromatofoor::bounded_context::Command;

impl Command<TokenStatusList> for CreateStatusList {
    type Error = TokenStatusListError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &TokenStatusList,
    ) -> Result<Vec<TokenStatusListEvent>, Self::Error> {
        Ok(vec![TokenStatusListEvent::StatusListCreated(
            StatusListCreated {
                status_list_id: self.status_list_id,
                bitstring_size: self.bitstring_size,
            },
        )])
    }
}
