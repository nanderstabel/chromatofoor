use crate::generated::domain::token_status_list::aggregate::TokenStatusList;
use crate::generated::domain::token_status_list::event::CredentialIndexRevoked;
use chromatofoor::bounded_context::Event;

impl Event<TokenStatusList> for CredentialIndexRevoked {
    fn evolve(&self, mut state: TokenStatusList) -> TokenStatusList {
        if let TokenStatusList::Created(inner) = &mut state {
            inner.revoked_count = Some(inner.revoked_count.unwrap_or(0) + 1);
        }
        state
    }
}
