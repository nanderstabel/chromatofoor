use std::{collections::BTreeMap, convert::Infallible, fmt, sync::Arc};

fn default_problem_type() -> String {
    "about:blank".to_owned()
}

/// Transport-agnostic [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) Problem Details document
/// describing a domain, validation, authorization, or infrastructure failure.
///
/// Presentation layers (`chromatofoor-http`, `chromatofoor-graphql`, `chromatofoor-mcp`,
/// `chromatofoor-tauri`, `chromatofoor-tui`) project `ProblemDetails` into their native error
/// envelopes (such as `application/problem+json`, GraphQL `extensions`, or JSON-RPC `error.data`)
/// without requiring HTTP dependencies in [`chromatofoor`](crate).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProblemDetails {
    /// URI reference identifying the problem type (`"type"`; defaults to `"about:blank"`).
    #[serde(rename = "type", default = "default_problem_type")]
    pub type_uri: String,
    /// Short, human-readable summary of the problem type (`"title"`).
    pub title: String,
    /// HTTP status code applicable to this occurrence (`"status"`).
    pub status: u16,
    /// Human-readable explanation specific to this occurrence (`"detail"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// URI reference identifying the specific occurrence or target (`"instance"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Machine-readable error or invariant slug (`"code"` extension member).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Arbitrary RFC 9457 problem-type extension members flattened into the top-level JSON object.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, serde_json::Value>,
}

impl ProblemDetails {
    /// Creates a [`ProblemDetails`] document with `type = "about:blank"`.
    #[must_use]
    pub fn new(status: u16, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            type_uri: default_problem_type(),
            title: title.into(),
            status,
            detail: Some(detail.into()),
            instance: None,
            code: None,
            extensions: BTreeMap::new(),
        }
    }

    /// Creates a `422 Unprocessable Content` [`ProblemDetails`] for a violated ESDM invariant.
    #[must_use]
    pub fn invariant(scope_topic: &str, invariant: &str, rule: impl Into<String>) -> Self {
        let trimmed_topic = scope_topic.trim_matches('/');
        Self {
            type_uri: format!("urn:chromatofoor:invariant:{trimmed_topic}:{invariant}"),
            title: "Invariant Violation".to_owned(),
            status: 422,
            detail: Some(rule.into()),
            instance: Some(scope_topic.to_owned()),
            code: Some(invariant.to_owned()),
            extensions: BTreeMap::new(),
        }
    }

    /// Creates a `422 Unprocessable Content` [`ProblemDetails`] for payload validation errors.
    #[must_use]
    pub fn validation(detail: impl Into<String>) -> Self {
        Self {
            type_uri: "urn:chromatofoor:error:validation".to_owned(),
            title: "Validation Failed".to_owned(),
            status: 422,
            detail: Some(detail.into()),
            instance: None,
            code: Some("validation-failed".to_owned()),
            extensions: BTreeMap::new(),
        }
    }

    /// Creates a `404 Not Found` [`ProblemDetails`] for missing resources or read-model records.
    #[must_use]
    pub fn not_found(detail: impl Into<String>) -> Self {
        Self {
            type_uri: "urn:chromatofoor:error:not-found".to_owned(),
            title: "Resource Not Found".to_owned(),
            status: 404,
            detail: Some(detail.into()),
            instance: None,
            code: Some("not-found".to_owned()),
            extensions: BTreeMap::new(),
        }
    }

    /// Creates a `400 Bad Request` [`ProblemDetails`] for malformed inputs or routing arguments.
    #[must_use]
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self {
            type_uri: "urn:chromatofoor:error:bad-request".to_owned(),
            title: "Bad Request".to_owned(),
            status: 400,
            detail: Some(detail.into()),
            instance: None,
            code: Some("bad-request".to_owned()),
            extensions: BTreeMap::new(),
        }
    }

    /// Sets the RFC 9457 `type` URI.
    #[must_use]
    pub fn with_type(mut self, uri: impl Into<String>) -> Self {
        self.type_uri = uri.into();
        self
    }

    /// Sets the RFC 9457 `instance` URI or topic path.
    #[must_use]
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }

    /// Sets `instance` only when not already populated.
    #[must_use]
    pub fn with_default_instance(mut self, instance: impl Into<String>) -> Self {
        if self.instance.is_none() {
            self.instance = Some(instance.into());
        }
        self
    }

    /// Sets the machine-readable `code` extension field.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Inserts an arbitrary RFC 9457 extension member into [`Self::extensions`].
    #[must_use]
    pub fn with_extension(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.extensions.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for ProblemDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) if !detail.is_empty() => f.write_str(detail),
            _ => f.write_str(&self.title),
        }
    }
}

impl std::error::Error for ProblemDetails {}

impl architecturekit::Schema for ProblemDetails {
    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "description": "RFC 9457 Problem Details response payload.",
            "properties": {
                "type": { "type": "string" },
                "title": { "type": "string" },
                "status": { "type": "integer" },
                "detail": { "type": "string" },
                "instance": { "type": "string" },
                "code": { "type": "string" }
            },
            "required": ["type", "title", "status"],
            "additionalProperties": true
        })
    }
}

impl architecturekit::Categorized for ProblemDetails {
    fn category(&self) -> Option<architecturekit::Category> {
        match self.status {
            422 => Some(architecturekit::Category::Domain),
            409 | 503 => Some(architecturekit::Category::Transient),
            500 => Some(architecturekit::Category::Permanent),
            _ => None,
        }
    }
}

/// Converts a domain, validation, or query error into an RFC 9457 [`ProblemDetails`] document.
pub trait Problem: fmt::Display {
    /// Projects this error into structured [`ProblemDetails`].
    fn to_problem(&self) -> ProblemDetails;
}

impl Problem for ProblemDetails {
    fn to_problem(&self) -> ProblemDetails {
        self.clone()
    }
}

impl Problem for str {
    fn to_problem(&self) -> ProblemDetails {
        ProblemDetails::new(422, "Domain Rule Violation", self)
            .with_type("urn:chromatofoor:error:domain")
    }
}

impl Problem for String {
    fn to_problem(&self) -> ProblemDetails {
        self.as_str().to_problem()
    }
}

impl Problem for Infallible {
    fn to_problem(&self) -> ProblemDetails {
        match *self {}
    }
}

impl Problem for architecturekit::DomainError {
    fn to_problem(&self) -> ProblemDetails {
        ProblemDetails::new(422, "Domain Rule Violation", self.to_string())
            .with_type("urn:chromatofoor:error:domain")
    }
}

impl Problem for architecturekit::ViewError {
    fn to_problem(&self) -> ProblemDetails {
        ProblemDetails::new(500, "Read Model Failure", self.to_string())
            .with_type("urn:chromatofoor:error:view")
            .with_code("view-error")
    }
}

impl Problem for validator::ValidationErrors {
    fn to_problem(&self) -> ProblemDetails {
        let mut problem = ProblemDetails::validation(self.to_string());
        if let Ok(errors_val) = serde_json::to_value(self) {
            problem = problem.with_extension("errors", errors_val);
        }
        problem
    }
}

impl<T: Problem + ?Sized> Problem for &T {
    fn to_problem(&self) -> ProblemDetails {
        (**self).to_problem()
    }
}

impl<T: Problem + ?Sized> Problem for Box<T> {
    fn to_problem(&self) -> ProblemDetails {
        (**self).to_problem()
    }
}

impl<T: Problem + ?Sized> Problem for Arc<T> {
    fn to_problem(&self) -> ProblemDetails {
        (**self).to_problem()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn problem_details_flattens_and_roundtrips_rfc9457_extension_members() {
        let problem = ProblemDetails::invariant(
            "/bank-account-example/finance/account",
            "insufficient-funds",
            "Account balance must cover the requested withdrawal amount.",
        )
        .with_extension("available_balance", 30)
        .with_extension("requested_amount", 50);

        let serialized = serde_json::to_value(&problem).unwrap();
        assert_eq!(
            serialized,
            json!({
                "type": "urn:chromatofoor:invariant:bank-account-example/finance/account:insufficient-funds",
                "title": "Invariant Violation",
                "status": 422,
                "detail": "Account balance must cover the requested withdrawal amount.",
                "instance": "/bank-account-example/finance/account",
                "code": "insufficient-funds",
                "available_balance": 30,
                "requested_amount": 50
            })
        );

        let decoded: ProblemDetails = serde_json::from_value(serialized).unwrap();
        assert_eq!(decoded, problem);
    }
}
