#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Topic(String);

impl Topic {
    #[must_use]
    pub fn new(topic: impl Into<String>) -> Self {
        Self(topic.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Checks at compile time whether `child_topic` (`/<domain>/...`) belongs under
/// `domain_topic` (`/<domain>`).
#[must_use]
pub const fn topic_in_domain(domain_topic: &str, child_topic: &str) -> bool {
    let d = domain_topic.as_bytes();
    let c = child_topic.as_bytes();
    if c.len() <= d.len() {
        return false;
    }
    let mut i = 0;
    while i < d.len() {
        if d[i] != c[i] {
            return false;
        }
        i += 1;
    }
    c[d.len()] == b'/'
}
