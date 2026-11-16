use crate::generated::domain::token_status_list::aggregate::TokenStatusList;
use crate::generated::domain::token_status_list::event::StatusListCreated;
use chromatofoor::bounded_context::Event;

impl Event<TokenStatusList> for StatusListCreated {
    fn evolve(&self, _state: TokenStatusList) -> TokenStatusList {
        TokenStatusList::Created(
            crate::generated::domain::token_status_list::aggregate::TokenStatusListInner {
                status_list_id: Some(self.status_list_id.clone()),
                bitstring_size: Some(self.bitstring_size),
                revoked_count: Some(0),
            },
        )
    }
}
