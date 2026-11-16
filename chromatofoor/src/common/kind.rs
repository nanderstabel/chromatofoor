use crate::common::topic::Topic;

pub trait Kind: 'static {
    type Scope;
    const KIND: &'static str;
    const NAME: &'static str;
    const TOPIC: &'static str;
    const DESCRIPTION: &'static str = "";

    #[must_use]
    fn topic(&self) -> Topic {
        Topic::new(Self::TOPIC)
    }
}
