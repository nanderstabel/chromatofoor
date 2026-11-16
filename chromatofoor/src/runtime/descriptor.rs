use std::{future::Future, pin::Pin, sync::Arc};

use serde_json::Value;

use crate::{
    bounded_context::{
        ActorType, BoundedContext, Command, Query, ReadModel, command::CommandScope,
    },
    common::{IdSource, Identified, Kind},
    domain::Domain,
    integration::{DeliveryGuarantee, EventConsumer, Idempotency},
};

use super::error::{DispatchError, ensure_authorized_instance, ensure_authorized_type};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type CommandFn = Arc<
    dyn for<'a> Fn(
            &'a Arc<architecturekit::Store>,
            Value,
            Option<&'a str>,
        ) -> BoxFuture<
            'a,
            Result<Vec<architecturekit::eventsourcingdb::Event>, DispatchError>,
        > + Send
        + Sync,
>;

type QueryFn = Arc<
    dyn for<'a> Fn(Value, Option<&'a str>) -> BoxFuture<'a, Result<Value, DispatchError>>
        + Send
        + Sync,
>;

async fn execute_command<S: CommandScope, C: Command<S>>(
    store: &Arc<architecturekit::Store>,
    payload: Value,
    actor: Option<&str>,
) -> Result<Vec<architecturekit::eventsourcingdb::Event>, DispatchError> {
    let actor_name = ensure_authorized_type::<C>(actor)?;
    let cmd: C = serde_json::from_value(payload)?;
    ensure_authorized_instance(&cmd, actor_name)?;
    S::execute(store, cmd)
        .await
        .map_err(|e| DispatchError::Execution(e.to_string()))
}

async fn execute_query<R: ReadModel, Q: Query<R>>(
    read_model: Arc<R>,
    params: Value,
    actor: Option<&str>,
) -> Result<Value, DispatchError> {
    let actor_name = ensure_authorized_type::<Q>(actor)?;
    let query: Q = serde_json::from_value(params)?;
    ensure_authorized_instance(&query, actor_name)?;
    let output = query
        .execute(&read_model)
        .await
        .map_err(|e| DispatchError::Execution(e.to_string()))?;
    Ok(serde_json::to_value(output)?)
}

/// Runtime metadata, JSON Schema, and dispatcher for a registered [`Command`].
#[derive(Clone)]
pub struct CommandDescriptor {
    /// Kebab-case name of the owning [`BoundedContext`].
    pub bounded_context: &'static str,
    /// Kebab-case name of the target [`Aggregate`](crate::bounded_context::Aggregate) or
    /// [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary).
    pub target: &'static str,
    /// ESDM kind of the target (`"aggregate"` or `"dynamic-consistency-boundary"`).
    pub target_kind: &'static str,
    /// Kebab-case name of the command.
    pub name: &'static str,
    /// Full topic path (`/<domain>/<bounded-context>/<target>/<command>`).
    pub topic: &'static str,
    /// Documentation string extracted from the command's Rust doc comments.
    pub description: &'static str,
    /// Allowed `(actor_name, ActorType)` pairs declared on the command.
    pub allowed_actors: &'static [(&'static str, ActorType)],
    /// Subject identification rules of the command's target scope.
    pub identified_by: &'static [IdSource],
    /// JSON Schema of the command payload derived via [`architecturekit::Schema`].
    pub schema: Value,
    handler: CommandFn,
}

impl std::fmt::Debug for CommandDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandDescriptor")
            .field("bounded_context", &self.bounded_context)
            .field("target", &self.target)
            .field("target_kind", &self.target_kind)
            .field("name", &self.name)
            .field("topic", &self.topic)
            .field("description", &self.description)
            .field("allowed_actors", &self.allowed_actors)
            .field("identified_by", &self.identified_by)
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

impl CommandDescriptor {
    /// Builds a [`CommandDescriptor`] from the compile-time metadata of command `C` and its
    /// target consistency boundary `S`.
    #[must_use]
    pub fn of<S, C>() -> Self
    where
        S: CommandScope,
        S::Scope: BoundedContext,
        C: Command<S>,
    {
        Self {
            bounded_context: <S::Scope as Kind>::NAME,
            target: S::NAME,
            target_kind: S::KIND,
            name: C::NAME,
            topic: C::TOPIC,
            description: C::DESCRIPTION,
            allowed_actors: C::ALLOWED_ACTORS,
            identified_by: <S as Identified>::IDENTIFIED_BY,
            schema: C::schema(),
            handler: Arc::new(|store, payload, actor| {
                Box::pin(execute_command::<S, C>(store, payload, actor))
            }),
        }
    }

    /// Authorizes, deserializes, and executes this command against `store`.
    pub async fn execute(
        &self,
        store: &Arc<architecturekit::Store>,
        payload: Value,
        actor: Option<&str>,
    ) -> Result<Vec<architecturekit::eventsourcingdb::Event>, DispatchError> {
        (self.handler)(store, payload, actor).await
    }
}

/// Runtime metadata, JSON Schemas, and dispatcher for a registered [`Query`].
#[derive(Clone)]
pub struct QueryDescriptor {
    /// Kebab-case name of the owning [`BoundedContext`].
    pub bounded_context: &'static str,
    /// Kebab-case name of the target [`ReadModel`].
    pub read_model: &'static str,
    /// Kebab-case name of the query.
    pub name: &'static str,
    /// Full topic path (`/<domain>/<bounded-context>/<query>`).
    pub topic: &'static str,
    /// Documentation string extracted from the query's Rust doc comments.
    pub description: &'static str,
    /// Allowed `(actor_name, ActorType)` pairs declared on the query.
    pub allowed_actors: &'static [(&'static str, ActorType)],
    /// JSON Schema of the query parameters derived via [`architecturekit::Schema`].
    pub parameters_schema: Value,
    /// JSON Schema of the query output derived via [`architecturekit::Schema`].
    pub output_schema: Value,
    handler: QueryFn,
}

impl std::fmt::Debug for QueryDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryDescriptor")
            .field("bounded_context", &self.bounded_context)
            .field("read_model", &self.read_model)
            .field("name", &self.name)
            .field("topic", &self.topic)
            .field("description", &self.description)
            .field("allowed_actors", &self.allowed_actors)
            .field("parameters_schema", &self.parameters_schema)
            .field("output_schema", &self.output_schema)
            .finish_non_exhaustive()
    }
}

impl QueryDescriptor {
    /// Builds a [`QueryDescriptor`] from the compile-time metadata of query `Q` and its target
    /// read model `R`.
    #[must_use]
    pub fn of<R, Q>(read_model: Arc<R>) -> Self
    where
        R: ReadModel,
        Q: Query<R>,
    {
        Self {
            bounded_context: <R::Scope as Kind>::NAME,
            read_model: R::NAME,
            name: Q::NAME,
            topic: Q::TOPIC,
            description: Q::DESCRIPTION,
            allowed_actors: Q::ALLOWED_ACTORS,
            parameters_schema: Q::schema(),
            output_schema: <Q::Output as architecturekit::Schema>::schema(),
            handler: Arc::new(move |params, actor| {
                Box::pin(execute_query::<R, Q>(
                    Arc::clone(&read_model),
                    params,
                    actor,
                ))
            }),
        }
    }

    /// Authorizes, deserializes, and executes this query against its registered read model.
    pub async fn execute(
        &self,
        params: Value,
        actor: Option<&str>,
    ) -> Result<Value, DispatchError> {
        (self.handler)(params, actor).await
    }
}

/// Runtime metadata for a registered [`ReadModel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadModelDescriptor {
    /// Kebab-case name of the owning [`BoundedContext`].
    pub bounded_context: &'static str,
    /// Kebab-case name of the read model.
    pub name: &'static str,
    /// Full topic path (`/<domain>/<bounded-context>/<read-model>`).
    pub topic: &'static str,
    /// Documentation string extracted from the read model's Rust doc comments.
    pub description: &'static str,
    /// Topic paths of the events projected into this read model.
    pub events: &'static [&'static str],
}

impl ReadModelDescriptor {
    /// Builds a [`ReadModelDescriptor`] from the compile-time metadata of read model `R`.
    #[must_use]
    pub fn of<R: ReadModel>() -> Self {
        Self {
            bounded_context: <R::Scope as Kind>::NAME,
            name: R::NAME,
            topic: R::TOPIC,
            description: R::DESCRIPTION,
            events: R::EVENTS,
        }
    }
}

/// Runtime metadata for a registered domain-level [`EventConsumer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumerDescriptor {
    /// ESDM kind (`"policy"`, `"event-handler"`, or `"process-manager"`).
    pub kind: &'static str,
    /// Kebab-case name of the consumer.
    pub name: &'static str,
    /// Full topic path (`/<domain>/<consumer>`).
    pub topic: &'static str,
    /// Documentation string extracted from the consumer's Rust doc comments.
    pub description: &'static str,
    /// Configured delivery guarantee.
    pub delivery_guarantee: DeliveryGuarantee,
    /// Configured idempotency strategy, if any.
    pub idempotency: Option<Idempotency>,
}

impl ConsumerDescriptor {
    /// Builds a [`ConsumerDescriptor`] from the compile-time metadata of event consumer `C`.
    #[must_use]
    pub fn of<C>() -> Self
    where
        C: EventConsumer,
        C::Scope: Domain,
    {
        Self {
            kind: C::KIND,
            name: C::NAME,
            topic: C::TOPIC,
            description: C::DESCRIPTION,
            delivery_guarantee: C::DELIVERY_GUARANTEE,
            idempotency: C::IDEMPOTENCY,
        }
    }
}
