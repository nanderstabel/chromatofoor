use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

use crate::{
    bounded_context::{BoundedContext, Command, Query, ReadModel, command::CommandScope},
    common::{Problem, topic_in_domain},
    domain::Domain,
    integration::EventConsumer,
};

use super::{
    descriptor::{CommandDescriptor, ConsumerDescriptor, QueryDescriptor, ReadModelDescriptor},
    error::DispatchError,
};

type CommandKey = (&'static str, &'static str, &'static str);
type QueryKey = (&'static str, &'static str);

type ProjectionStarter = Box<
    dyn FnOnce(
            &Arc<architecturekit::Store>,
            &architecturekit::CancellationToken,
        ) -> architecturekit::ProjectionRun
        + Send,
>;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BoxPresentationFuture = Pin<Box<dyn Future<Output = Result<(), BoxError>> + Send>>;

/// Transport-agnostic presentation layer that can be attached to a [`DomainBuilder`] or
/// [`DomainRuntime`] and executed alongside other presentation layers via `.serve()`.
///
/// Built-in implementations include:
/// - `chromatofoor_http::HttpServer` (with optional `HttpProtocol` sub-adapters such as
///   `chromatofoor_graphql::GraphQL` and `chromatofoor_mcp::Mcp::http()`)
/// - `chromatofoor_mcp::Mcp::stdio()` (zero HTTP dependencies)
/// - `chromatofoor_tui::Tui`
///
/// Third-party crates can implement [`Presentation`] directly to attach custom transports
/// (such as gRPC, NATS, or CLI runners) to any [`DomainBuilder`] without requiring HTTP
/// dependencies in [`chromatofoor`](crate).
///
/// # Example
///
/// ```
/// # use std::sync::{
/// #     Arc,
/// #     atomic::{AtomicUsize, Ordering as AtomicOrdering},
/// # };
/// # use chromatofoor::{
/// #     InMemoryEventStore,
/// #     architecturekit::Store,
/// #     bounded_context::{Command, Event},
/// #     common::Identifier,
/// #     runtime::{DomainBuilder, DomainRuntime, Presentation},
/// #     serde_json::json,
/// # };
/// # #[chromatofoor::domain]
/// # pub struct TestDomain;
/// # #[chromatofoor::bounded_context(scope = TestDomain)]
/// # pub struct Billing;
/// # #[chromatofoor::actor(scope = Billing, type = human)]
/// # pub struct Accountant;
/// # #[chromatofoor::aggregate(scope = Billing, identified_by = state(invoice_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Invoice { pub invoice_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Invoice)]
/// # pub enum InvoiceEvent { InvoiceIssued { invoice_id: Identifier } }
/// # impl Event<Invoice> for InvoiceIssued {
/// #     fn evolve(&self, _state: Invoice) -> Invoice { Invoice { invoice_id: Some(self.invoice_id.clone()) } }
/// # }
/// # #[chromatofoor::command(scope = Invoice)]
/// # pub enum InvoiceCommand {
/// #     #[lifecycle(create)]
/// #     #[actors(Accountant)]
/// #     IssueInvoice { invoice_id: Identifier },
/// # }
/// # impl Command<Invoice> for IssueInvoice {
/// #     type Error = &'static str;
/// #     async fn decide(self, _state: &Invoice) -> Result<Vec<InvoiceEvent>, Self::Error> {
/// #         Ok(vec![InvoiceIssued { invoice_id: self.invoice_id }.into()])
/// #     }
/// # }
/// struct CustomPresentation {
///     label: &'static str,
///     invoice_id: String,
///     counter: Arc<AtomicUsize>,
/// }
///
/// impl Presentation for CustomPresentation {
///     fn name(&self) -> &'static str {
///         self.label
///     }
///
///     async fn serve(
///         self,
///         runtime: Arc<DomainRuntime>,
///     ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///         runtime
///             .dispatch_command(
///                 "billing",
///                 "invoice",
///                 "issue-invoice",
///                 json!({ "invoice_id": self.invoice_id }),
///                 Some("accountant"),
///             )
///             .await?;
///         self.counter.fetch_add(1, AtomicOrdering::SeqCst);
///         Ok(())
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://billing.local").build());
/// let counter = Arc::new(AtomicUsize::new(0));
/// let runtime = DomainBuilder::<TestDomain>::new(store)
///     .register_command::<IssueInvoice, _>()
///     .attach(CustomPresentation {
///         label: "custom-a",
///         invoice_id: "inv-1".to_owned(),
///         counter: Arc::clone(&counter),
///     })
///     .attach(CustomPresentation {
///         label: "custom-b",
///         invoice_id: "inv-2".to_owned(),
///         counter: Arc::clone(&counter),
///     })
///     .build()
///     .await
///     .unwrap();
///
/// assert_eq!(runtime.attached_presentations(), vec!["custom-a", "custom-b"]);
/// assert_eq!(runtime.actors(), vec!["accountant"]);
/// runtime.serve().await.unwrap();
/// assert_eq!(counter.load(AtomicOrdering::SeqCst), 2);
/// # });
/// ```
pub trait Presentation: Send + 'static {
    /// Identifier for this presentation adapter (e.g. `"http"`, `"mcp-stdio"`, `"tui"`).
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Runs this presentation adapter against the shared [`DomainRuntime`].
    fn serve(
        self,
        runtime: Arc<DomainRuntime>,
    ) -> impl Future<Output = Result<(), BoxError>> + Send;
}

trait ErasedPresentation: Send {
    fn name(&self) -> &'static str;
    fn serve_boxed(self: Box<Self>, runtime: Arc<DomainRuntime>) -> BoxPresentationFuture;
}

impl<P: Presentation> ErasedPresentation for P {
    fn name(&self) -> &'static str {
        Presentation::name(self)
    }

    fn serve_boxed(self: Box<Self>, runtime: Arc<DomainRuntime>) -> BoxPresentationFuture {
        Box::pin((*self).serve(runtime))
    }
}

/// Builder that registers commands, read models, queries, domain-level [`EventConsumer`]s,
/// and [`Presentation`] layers into a running [`DomainRuntime`].
pub struct DomainBuilder<D: Domain> {
    store: Arc<architecturekit::Store>,
    commands: BTreeMap<CommandKey, CommandDescriptor>,
    queries: BTreeMap<QueryKey, QueryDescriptor>,
    read_models: Vec<ReadModelDescriptor>,
    consumers: Vec<ConsumerDescriptor>,
    starters: Vec<ProjectionStarter>,
    presentations: Vec<Box<dyn ErasedPresentation>>,
    _marker: PhantomData<D>,
}

impl<D: Domain> DomainBuilder<D> {
    /// Creates a new domain runtime builder for [`Domain`] `D` backed by `store`.
    #[must_use]
    pub fn new(store: Arc<architecturekit::Store>) -> Self {
        Self {
            store,
            commands: BTreeMap::new(),
            queries: BTreeMap::new(),
            read_models: Vec::new(),
            consumers: Vec::new(),
            starters: Vec::new(),
            presentations: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Registers a command `C` whose target consistency unit `S` (`Aggregate` or
    /// `DynamicConsistencyBoundary`) belongs to a [`BoundedContext`](crate::bounded_context::BoundedContext)
    /// in [`Domain`] `D`.
    #[must_use]
    pub fn register_command<C, S>(mut self) -> Self
    where
        S: CommandScope,
        S::Scope: BoundedContext,
        C: Command<S, Error: Problem>,
    {
        const {
            assert!(
                topic_in_domain(D::TOPIC, S::TOPIC),
                "command target does not belong to domain"
            );
        }
        let desc = CommandDescriptor::of::<S, C>();
        let key = (desc.bounded_context, desc.target, desc.name);
        assert!(
            !self.commands.contains_key(&key),
            "duplicate command registration for topic `{}`",
            desc.topic,
        );
        self.commands.insert(key, desc);
        self
    }

    /// Registers a [`ReadModel`] `R` belonging to [`Domain`] `D` so its projection is started
    /// when the domain runtime builds.
    #[must_use]
    pub fn register_read_model<R>(mut self, read_model: Arc<R>) -> Self
    where
        R: ReadModel,
    {
        const {
            assert!(
                topic_in_domain(D::TOPIC, R::TOPIC),
                "read model does not belong to domain"
            );
        }
        assert!(
            !self.read_models.iter().any(|rm| rm.topic == R::TOPIC),
            "duplicate read model registration for topic `{}`",
            R::TOPIC,
        );
        self.read_models.push(ReadModelDescriptor::of::<R>());
        self.starters.push(Box::new(move |store, token| {
            store.start_projection(
                architecturekit::Subjects::tree(architecturekit::Subject::new(D::TOPIC)),
                read_model,
                token.clone(),
                architecturekit::ProjectionOptions::named(format!("{}:{}", R::KIND, R::TOPIC)),
            )
        }));
        self
    }

    /// Registers a [`Query`] `Q` against [`ReadModel`] `R` belonging to [`Domain`] `D`.
    #[must_use]
    pub fn register_query<Q, R>(mut self, read_model: Arc<R>) -> Self
    where
        R: ReadModel,
        Q: Query<R, Error: Problem>,
    {
        const {
            assert!(
                topic_in_domain(D::TOPIC, R::TOPIC),
                "query read model does not belong to domain"
            );
        }
        let desc = QueryDescriptor::of::<R, Q>(read_model);
        let key = (desc.bounded_context, desc.name);
        assert!(
            !self.queries.contains_key(&key),
            "duplicate query registration for topic `{}`",
            desc.topic,
        );
        self.queries.insert(key, desc);
        self
    }

    /// Registers a domain-scoped [`EventConsumer`] (`Policy`, `EventHandler`, or `ProcessManager`)
    /// belonging to [`Domain`] `D`.
    #[must_use]
    pub fn register_consumer<C>(mut self, consumer: C) -> Self
    where
        C: EventConsumer,
    {
        const {
            assert!(
                topic_in_domain(D::TOPIC, C::TOPIC),
                "event consumer does not belong to domain"
            );
        }
        assert!(
            !self
                .consumers
                .iter()
                .any(|c| c.kind == C::KIND && c.topic == C::TOPIC),
            "duplicate `{}` consumer registration for topic `{}`",
            C::KIND,
            C::TOPIC,
        );
        self.consumers.push(ConsumerDescriptor::of::<C>());
        self.starters.push(Box::new(move |store, token| {
            consumer.start(Arc::clone(store), token.clone())
        }));
        self
    }

    /// Attaches a [`Presentation`] layer (such as `Mcp::stdio()`, `Tui::new()`, `HttpServer`, or
    /// a custom third-party adapter) to be run when [`Self::serve`] or [`DomainRuntime::serve`]
    /// is invoked.
    #[must_use]
    pub fn attach<P: Presentation>(mut self, presentation: P) -> Self {
        self.presentations.push(Box::new(presentation));
        self
    }

    /// Starts all registered read-model projections and event consumers, waits for their initial
    /// catch-up pass, and returns the [`DomainRuntime`].
    pub async fn build(self) -> Result<DomainRuntime, BoxError> {
        let token = architecturekit::CancellationToken::new();
        let mut runs = Vec::with_capacity(self.starters.len());
        for starter in self.starters {
            runs.push(starter(&self.store, &token));
        }

        if !runs.is_empty() {
            let refs: Vec<&architecturekit::ProjectionRun> = runs.iter().collect();
            architecturekit::wait_caught_up(&refs).await?;
        }

        Ok(DomainRuntime {
            domain_name: D::NAME,
            domain_topic: D::TOPIC,
            domain_description: D::DESCRIPTION,
            store: self.store,
            commands: self.commands.into_values().collect(),
            queries: self.queries.into_values().collect(),
            read_models: self.read_models,
            consumers: self.consumers,
            presentations: Arc::new(Mutex::new(self.presentations)),
            supervisor: Arc::new(ProjectionSupervisor { runs, token }),
        })
    }

    /// Builds the [`DomainRuntime`] and concurrently serves all attached [`Presentation`] layers.
    pub async fn serve(self) -> Result<(), BoxError> {
        let runtime = self.build().await?;
        runtime.serve().await
    }
}

struct ProjectionSupervisor {
    runs: Vec<architecturekit::ProjectionRun>,
    token: architecturekit::CancellationToken,
}

impl Drop for ProjectionSupervisor {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Projection forwarder used by [`DomainRuntime::subscribe_events`] to stream raw events over an
/// unbounded channel.
struct RawEventForwarder {
    tx: tokio::sync::mpsc::UnboundedSender<architecturekit::eventsourcingdb::Event>,
    token: architecturekit::CancellationToken,
}

impl architecturekit::Projection for RawEventForwarder {
    type Event = architecturekit::eventsourcingdb::Event;
    type Error = std::convert::Infallible;

    fn apply(
        &self,
        event: architecturekit::Envelope<Self::Event>,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        if self.tx.is_closed() || self.tx.send(event.data).is_err() {
            self.token.cancel();
        }
        std::future::ready(Ok(()))
    }
}

/// Shared, transport-agnostic domain runtime that presentation layers (`chromatofoor-http`,
/// `chromatofoor-graphql`, `chromatofoor-mcp`, `chromatofoor-tui`, `chromatofoor-tauri`, or
/// custom third-party adapters) connect to concurrently.
#[derive(Clone)]
pub struct DomainRuntime {
    domain_name: &'static str,
    domain_topic: &'static str,
    domain_description: &'static str,
    store: Arc<architecturekit::Store>,
    commands: Vec<CommandDescriptor>,
    queries: Vec<QueryDescriptor>,
    read_models: Vec<ReadModelDescriptor>,
    consumers: Vec<ConsumerDescriptor>,
    presentations: Arc<Mutex<Vec<Box<dyn ErasedPresentation>>>>,
    supervisor: Arc<ProjectionSupervisor>,
}

impl DomainRuntime {
    /// Kebab-case name of the [`Domain`].
    #[must_use]
    pub fn domain_name(&self) -> &'static str {
        self.domain_name
    }

    /// Root topic of the [`Domain`] (`/<domain>`).
    #[must_use]
    pub fn domain_topic(&self) -> &'static str {
        self.domain_topic
    }

    /// Description of the [`Domain`] extracted from its Rust doc comment.
    #[must_use]
    pub fn domain_description(&self) -> &'static str {
        self.domain_description
    }

    /// Returns a reference to the underlying [`architecturekit::Store`].
    #[must_use]
    pub fn store(&self) -> &Arc<architecturekit::Store> {
        &self.store
    }

    /// Returns descriptors and JSON Schemas for all registered commands, sorted deterministically.
    #[must_use]
    pub fn commands(&self) -> &[CommandDescriptor] {
        &self.commands
    }

    /// Returns descriptors and JSON Schemas for all registered queries, sorted deterministically.
    #[must_use]
    pub fn queries(&self) -> &[QueryDescriptor] {
        &self.queries
    }

    /// Returns descriptors for all registered read models.
    #[must_use]
    pub fn read_models(&self) -> &[ReadModelDescriptor] {
        &self.read_models
    }

    /// Returns descriptors for all registered domain-scoped event consumers.
    #[must_use]
    pub fn consumers(&self) -> &[ConsumerDescriptor] {
        &self.consumers
    }

    /// Returns all distinct actor names declared across registered commands and queries, sorted
    /// alphabetically.
    #[must_use]
    pub fn actors(&self) -> Vec<&'static str> {
        let mut set = BTreeSet::new();
        for cmd in &self.commands {
            for (actor, _) in cmd.allowed_actors {
                set.insert(*actor);
            }
        }
        for q in &self.queries {
            for (actor, _) in q.allowed_actors {
                set.insert(*actor);
            }
        }
        set.into_iter().collect()
    }

    /// Returns all commands that `actor` is authorized to invoke.
    #[must_use]
    pub fn commands_for_actor(&self, actor: &str) -> Vec<&CommandDescriptor> {
        self.commands
            .iter()
            .filter(|c| c.allowed_actors.iter().any(|(a, _)| *a == actor))
            .collect()
    }

    /// Returns all queries that `actor` is authorized to execute.
    #[must_use]
    pub fn queries_for_actor(&self, actor: &str) -> Vec<&QueryDescriptor> {
        self.queries
            .iter()
            .filter(|q| q.allowed_actors.iter().any(|(a, _)| *a == actor))
            .collect()
    }

    /// Builds a structured JSON catalog of the domain, including registered actors, commands,
    /// queries, read models, and event consumers.
    #[must_use]
    pub fn catalog_json(&self) -> Value {
        let commands: Vec<Value> = self
            .commands
            .iter()
            .map(|c| {
                let actors: Vec<&str> = c.allowed_actors.iter().map(|(a, _)| *a).collect();
                json!({
                    "kind": "command",
                    "bounded_context": c.bounded_context,
                    "target": c.target,
                    "target_kind": c.target_kind,
                    "name": c.name,
                    "topic": c.topic,
                    "description": c.description,
                    "allowed_actors": actors,
                    "schema": c.schema,
                })
            })
            .collect();

        let queries: Vec<Value> = self
            .queries
            .iter()
            .map(|q| {
                let actors: Vec<&str> = q.allowed_actors.iter().map(|(a, _)| *a).collect();
                json!({
                    "kind": "query",
                    "bounded_context": q.bounded_context,
                    "read_model": q.read_model,
                    "name": q.name,
                    "topic": q.topic,
                    "description": q.description,
                    "allowed_actors": actors,
                    "parameters_schema": q.parameters_schema,
                    "output_schema": q.output_schema,
                })
            })
            .collect();

        let read_models: Vec<Value> = self
            .read_models
            .iter()
            .map(|rm| {
                json!({
                    "kind": "read-model",
                    "bounded_context": rm.bounded_context,
                    "name": rm.name,
                    "topic": rm.topic,
                    "description": rm.description,
                    "events": rm.events,
                })
            })
            .collect();

        let consumers: Vec<Value> = self
            .consumers
            .iter()
            .map(|c| {
                json!({
                    "kind": c.kind,
                    "name": c.name,
                    "topic": c.topic,
                    "description": c.description,
                })
            })
            .collect();

        json!({
            "domain": self.domain_name,
            "topic": self.domain_topic,
            "description": self.domain_description,
            "actors": self.actors(),
            "commands": commands,
            "queries": queries,
            "read_models": read_models,
            "consumers": consumers,
        })
    }

    /// Attaches a [`Presentation`] layer to this runtime so it will be executed when
    /// [`Self::serve`] is called.
    #[must_use]
    pub fn attach<P: Presentation>(self, presentation: P) -> Self {
        if let Ok(mut guard) = self.presentations.lock() {
            guard.push(Box::new(presentation));
        }
        self
    }

    /// Returns the names of the currently attached [`Presentation`] adapters.
    #[must_use]
    pub fn attached_presentations(&self) -> Vec<&'static str> {
        self.presentations
            .lock()
            .map(|guard| guard.iter().map(|p| p.name()).collect())
            .unwrap_or_default()
    }

    /// Runs all attached [`Presentation`] layers concurrently against an `Arc<DomainRuntime>`.
    ///
    /// If a single presentation is attached, it is awaited directly on the current task. If
    /// multiple presentations are attached, they are spawned onto a [`tokio::task::JoinSet`] and
    /// supervised together until completion or the first error.
    pub async fn serve(self) -> Result<(), BoxError> {
        let presentations = self
            .presentations
            .lock()
            .map(|mut guard| std::mem::take(&mut *guard))
            .unwrap_or_default();

        let shared = Arc::new(self);
        match presentations.len() {
            0 => Ok(()),
            1 => {
                let mut iter = presentations.into_iter();
                if let Some(p) = iter.next() {
                    p.serve_boxed(shared).await?;
                }
                Ok(())
            }
            _ => {
                let mut set = tokio::task::JoinSet::new();
                for p in presentations {
                    let rt = Arc::clone(&shared);
                    set.spawn(async move { p.serve_boxed(rt).await });
                }
                while let Some(joined) = set.join_next().await {
                    match joined {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => {
                            set.abort_all();
                            return Err(err);
                        }
                        Err(join_err) => {
                            set.abort_all();
                            return Err(Box::new(join_err));
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Waits until all supervised read-model projections and event consumers have caught up with
    /// the latest events in the store.
    pub async fn wait_caught_up(&self) -> Result<(), BoxError> {
        if !self.supervisor.runs.is_empty() {
            let refs: Vec<&architecturekit::ProjectionRun> = self.supervisor.runs.iter().collect();
            architecturekit::wait_caught_up(&refs).await?;
        }
        Ok(())
    }

    /// Dynamically authorizes, deserializes, and executes a command by its
    /// `(bounded_context, target, command)` coordinates.
    pub async fn dispatch_command(
        &self,
        bounded_context: &str,
        target: &str,
        command: &str,
        payload: Value,
        actor: Option<&str>,
    ) -> Result<Vec<architecturekit::eventsourcingdb::Event>, DispatchError> {
        let Some(desc) = self.commands.iter().find(|c| {
            c.bounded_context == bounded_context && c.target == target && c.name == command
        }) else {
            if !self.has_bounded_context(bounded_context) {
                return Err(DispatchError::BoundedContextNotFound(
                    bounded_context.to_owned(),
                ));
            }
            return Err(DispatchError::CommandNotFound {
                bounded_context: bounded_context.to_owned(),
                target: target.to_owned(),
                command: command.to_owned(),
            });
        };
        desc.execute(&self.store, payload, actor).await
    }

    /// Dynamically authorizes, deserializes, and executes a query by its
    /// `(bounded_context, query)` coordinates.
    pub async fn dispatch_query(
        &self,
        bounded_context: &str,
        query: &str,
        params: Value,
        actor: Option<&str>,
    ) -> Result<Value, DispatchError> {
        let Some(desc) = self
            .queries
            .iter()
            .find(|q| q.bounded_context == bounded_context && q.name == query)
        else {
            if !self.has_bounded_context(bounded_context) {
                return Err(DispatchError::BoundedContextNotFound(
                    bounded_context.to_owned(),
                ));
            }
            return Err(DispatchError::QueryNotFound {
                bounded_context: bounded_context.to_owned(),
                query: query.to_owned(),
            });
        };
        desc.execute(params, actor).await
    }

    /// Subscribes to all raw events under this domain's subject tree (useful for SSE streams
    /// and live TUI event logs).
    #[must_use]
    pub fn subscribe_events(
        &self,
    ) -> tokio::sync::mpsc::UnboundedReceiver<architecturekit::eventsourcingdb::Event> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let topic = self.domain_topic;
        let sub_token = architecturekit::CancellationToken::new();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let parent_token = self.supervisor.token.clone();
            let cancel_token = sub_token.clone();
            let closed_tx = tx.clone();
            handle.spawn(async move {
                tokio::select! {
                    () = parent_token.cancelled() => cancel_token.cancel(),
                    () = closed_tx.closed() => cancel_token.cancel(),
                }
            });
        }
        let _run = self.store.start_projection(
            architecturekit::Subjects::tree(architecturekit::Subject::new(topic)),
            Arc::new(RawEventForwarder {
                tx,
                token: sub_token.clone(),
            }),
            sub_token,
            architecturekit::ProjectionOptions::named(format!("{topic}-subscribe-events")),
        );
        rx
    }

    fn has_bounded_context(&self, bounded_context: &str) -> bool {
        self.commands
            .iter()
            .any(|c| c.bounded_context == bounded_context)
            || self
                .queries
                .iter()
                .any(|q| q.bounded_context == bounded_context)
            || self
                .read_models
                .iter()
                .any(|rm| rm.bounded_context == bounded_context)
    }
}
