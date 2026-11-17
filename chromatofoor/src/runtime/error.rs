use crate::bounded_context::Authorized;
use crate::common::{Kind, Problem, ProblemDetails};

/// Error returned when dynamically dispatching a command or query via
/// [`DomainRuntime`](super::DomainRuntime).
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The requested bounded context is not registered in the runtime.
    #[error("bounded context `{0}` is not registered")]
    BoundedContextNotFound(String),

    /// The requested command is not registered under the specified bounded context and target.
    #[error("command `{bounded_context}/{target}/{command}` is not registered")]
    CommandNotFound {
        bounded_context: String,
        target: String,
        command: String,
    },

    /// The requested query is not registered under the specified bounded context.
    #[error("query `{bounded_context}/{query}` is not registered")]
    QueryNotFound {
        bounded_context: String,
        query: String,
    },

    /// The caller's actor is not permitted to invoke the command or query.
    #[error("actor `{actor:?}` is not authorized to invoke `{target}`")]
    Unauthorized {
        target: &'static str,
        actor: Option<String>,
    },

    /// The JSON payload or query parameters did not match the expected schema.
    #[error("invalid payload: {0}")]
    InvalidPayload(#[from] serde_json::Error),

    /// Executing the command or query failed with structured RFC 9457 [`ProblemDetails`].
    #[error("{0}")]
    Execution(Box<ProblemDetails>),
}

impl DispatchError {
    /// Projects this dispatch error into an RFC 9457 [`ProblemDetails`] document.
    #[must_use]
    pub fn to_problem(&self) -> ProblemDetails {
        match self {
            Self::BoundedContextNotFound(bc) => {
                ProblemDetails::new(404, "Bounded Context Not Found", self.to_string())
                    .with_type("urn:chromatofoor:error:bounded-context-not-found")
                    .with_instance(format!("/{bc}"))
                    .with_code("bounded-context-not-found")
            }
            Self::CommandNotFound {
                bounded_context,
                target,
                command,
            } => ProblemDetails::new(404, "Command Not Found", self.to_string())
                .with_type("urn:chromatofoor:error:command-not-found")
                .with_instance(format!("/{bounded_context}/{target}/{command}"))
                .with_code("command-not-found"),
            Self::QueryNotFound {
                bounded_context,
                query,
            } => ProblemDetails::new(404, "Query Not Found", self.to_string())
                .with_type("urn:chromatofoor:error:query-not-found")
                .with_instance(format!("/{bounded_context}/{query}"))
                .with_code("query-not-found"),
            Self::Unauthorized { target, .. } => {
                ProblemDetails::new(403, "Forbidden", self.to_string())
                    .with_type("urn:chromatofoor:error:forbidden")
                    .with_instance((*target).to_owned())
                    .with_code("unauthorized")
            }
            Self::InvalidPayload(err) => {
                ProblemDetails::new(400, "Invalid Request Payload", err.to_string())
                    .with_type("urn:chromatofoor:error:invalid-payload")
                    .with_code("invalid-payload")
            }
            Self::Execution(problem) => (**problem).clone(),
        }
    }
}

impl Problem for DispatchError {
    fn to_problem(&self) -> ProblemDetails {
        Self::to_problem(self)
    }
}

/// Pre-screens that `actor` is present and listed in `T`'s type-level allowed actors before
/// deserializing the payload.
///
/// Authorization is fail-closed: `actor = None` or an empty [`Authorized::ALLOWED_ACTORS`]
/// returns [`DispatchError::Unauthorized`].
pub(super) fn ensure_authorized_type<T: Authorized + Kind>(
    actor: Option<&str>,
) -> Result<&str, DispatchError> {
    if let Some(actor_name) = actor
        && T::is_allowed_actor(actor_name)
    {
        return Ok(actor_name);
    }
    Err(DispatchError::Unauthorized {
        target: T::TOPIC,
        actor: actor.map(ToOwned::to_owned),
    })
}

/// Verifies that `actor_name` is authorized to invoke the deserialized command or query `instance`
/// (enforcing per-variant `#[actors(...)]` on command enums).
pub(super) fn ensure_authorized_instance<T: Authorized + Kind>(
    instance: &T,
    actor_name: &str,
) -> Result<(), DispatchError> {
    if instance.is_authorized_for(actor_name) {
        return Ok(());
    }
    Err(DispatchError::Unauthorized {
        target: T::TOPIC,
        actor: Some(actor_name.to_owned()),
    })
}
