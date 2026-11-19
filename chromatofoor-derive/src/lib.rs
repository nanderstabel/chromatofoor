use proc_macro::TokenStream;

mod bounded_context;
mod common;
mod domain;
mod integration;
mod schema;

/// Marks a type as a top-level ESDM `domain` (`Scope = ()`).
#[proc_macro_attribute]
pub fn domain(attr: TokenStream, item: TokenStream) -> TokenStream {
    domain::domain(attr, item)
}

/// Marks a type as an ESDM `subdomain` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this subdomain belongs to.
/// - `type = core | supporting | generic` *(required)*: Strategic classification of this subdomain.
/// - `bounded_contexts = [<BoundedContext>, ...]` *(required, non-empty)*: Bounded contexts
///   within `scope` covered by this subdomain.
#[proc_macro_attribute]
pub fn subdomain(attr: TokenStream, item: TokenStream) -> TokenStream {
    domain::subdomain(attr, item)
}

/// Marks a type as an ESDM `bounded-context` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this bounded context belongs to.
#[proc_macro_attribute]
pub fn bounded_context(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::bounded_context(attr, item)
}

/// Marks a struct or enum as an ESDM `aggregate` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this aggregate belongs to.
/// - `identified_by = generated(uuid) | state(<field>) | static("<value>")`
///   *(optional, defaults to `generated(uuid)`)*: How instances of this aggregate are
///   identified.
#[proc_macro_attribute]
pub fn aggregate(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::aggregate(attr, item)
}

/// Marks a struct or enum as an ESDM `dynamic-consistency-boundary` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context whose events this boundary
///   reasons over.
/// - `identified_by = [command_payload(<field>), ...] | static("<value>") | generated(uuid)`
///   *(optional, defaults to `generated(uuid)`)*: How instances of this boundary are
///   identified.
/// - `consults = [<EventA>, <EventB>, ...]` *(required)*: Events within the bounded context
///   folded into or published by this boundary.
#[proc_macro_attribute]
pub fn dynamic_consistency_boundary(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::dynamic_consistency_boundary(attr, item)
}

/// Marks an `enum` or `struct` as an ESDM `event`.
///
/// # Attributes
/// - `scope = <Aggregate>` *(when applied to an `enum`)*: Generates a struct for each named
///   or unit variant (or delegates to single-field tuple variants) and implements
///   `architecturekit::State` for the aggregate.
/// - `scope = <BoundedContext | Aggregate>` *(when applied to a `struct`)*: Marks the struct
///   as an ESDM event struct and derives `Clone`, `Debug`, `PartialEq`, `Serialize`,
///   `Deserialize`, and `architecturekit::Schema`.
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::event(attr, item)
}

/// Marks a type as an ESDM `domain-service` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this domain service belongs to.
#[proc_macro_attribute]
pub fn domain_service(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::domain_service(attr, item)
}

/// Marks a struct as an ESDM `actor` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this actor belongs to.
/// - `type = human | system` *(optional, defaults to `human`)*: Whether the actor is a
///   human role or an automated system trigger.
/// - `backed_by = [<ExternalSystem>, ...]` *(optional, only on `type = system`)*: External
///   systems that back this system actor.
#[proc_macro_attribute]
pub fn actor(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::actor(attr, item)
}

/// Marks an `enum` or `struct` as an ESDM `command` scoped to an `Aggregate` or
/// `DynamicConsistencyBoundary`.
///
/// # Attributes
/// - `scope = <Aggregate | DynamicConsistencyBoundary>` *(required)*: The consistency unit
///   these commands target.
/// - `creates` or `lifecycle = create | mutate` *(optional)*: Marks a command struct (or
///   all variants of a command enum) as an aggregate creation command. Individual enum
///   variants or structs can also be annotated with `#[creates]` or `#[lifecycle(create)]`.
/// - `actors = [<Actor>, ...]` *(optional)*: Default list of actors permitted to issue this
///   command (individual enum variants or structs can also add `#[actors(...)]`).
/// - `error = <ErrorType>` *(optional, enum only)*: Error type for the generated
///   `Command<Scope>` implementation on the enum (defaults to the first variant's `Error`).
/// - `validate` *(optional)*: Runs `validator::Validate::validate` before `Command::decide`.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::command(attr, item)
}

/// Marks a struct as an ESDM `query` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this query belongs to.
/// - `actors = [<Actor>, ...]` *(optional)*: The actors permitted to run this query.
#[proc_macro_attribute]
pub fn query(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::query(attr, item)
}

/// Marks a struct as an ESDM `entity` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this entity belongs to.
/// - `identified_by = schema(<field>) | static("<value>")` *(optional, defaults to
///   `schema(id)`)*: How instances of this entity are identified.
#[proc_macro_attribute]
pub fn entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::entity(attr, item)
}

/// Marks a struct as an ESDM `value-object` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context this value object belongs to.
#[proc_macro_attribute]
pub fn value_object(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::value_object(attr, item)
}

/// Marks a struct as an ESDM `read-model` scoped to a `BoundedContext`.
///
/// # Attributes
/// - `scope = <BoundedContext>` *(required)*: The bounded context that owns this read model.
/// - `events = [<EventA>, <EventB>, ...]` *(required)*: The events projected into this read
///   model via `Project<E>`.
#[proc_macro_attribute]
pub fn read_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    bounded_context::read_model(attr, item)
}

/// Marks a struct as an ESDM `policy` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this policy belongs to.
/// - `handles = [<Event>, ...]` *(required)*: Events that trigger this policy.
/// - `emits = [<Command>, ...]` *(required)*: Commands this policy may issue.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
#[proc_macro_attribute]
pub fn policy(attr: TokenStream, item: TokenStream) -> TokenStream {
    integration::policy(attr, item)
}

/// Marks a struct as an ESDM `event-handler` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this event handler belongs to.
/// - `handles = [<Event>, ...]` *(required)*: Events that trigger this handler.
/// - `external_systems = [<ExternalSystem>, ...]` *(optional)*: External systems invoked as side effects.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
/// - `error = <ErrorType>` *(optional, defaults to `architecturekit::DomainError`)*: Error type returned by `Handle<E>::handle`.
#[proc_macro_attribute]
pub fn event_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    integration::event_handler(attr, item)
}

/// Marks a struct as an ESDM `external-system` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this external system belongs to.
/// - `direction = inbound | outbound | bidirectional` *(required)*: Direction of interaction.
/// - `category = "..."` *(optional)*: Short category tag (e.g. `"payment"`, `"mail"`).
#[proc_macro_attribute]
pub fn external_system(attr: TokenStream, item: TokenStream) -> TokenStream {
    integration::external_system(attr, item)
}

/// Marks a struct as a top-level ESDM `context-mapping` (`Scope = ()`).
///
/// # Attributes
/// - Asymmetric patterns (`BoundedContext` or `ExternalSystem` endpoints):
///   - `type = customer_supplier, customer = <E1>, supplier = <E2>`
///   - `type = conformist, conformist = <E1>, upstream = <E2>`
///   - `type = anti_corruption_layer, downstream = <E1>, upstream = <E2>`
///   - `type = open_host_service, host = <E1>, consumer = <E2>`
///   - `type = published_language, publisher = <E1>, consumer = <E2>`
/// - Symmetric patterns (`BoundedContext`-only endpoints):
///   - `type = shared_kernel, participants = [<BC1>, <BC2>]`
///   - `type = partnership, participants = [<BC1>, <BC2>]`
///   - `type = separate_ways, participants = [<BC1>, <BC2>]`
#[proc_macro_attribute]
pub fn context_mapping(attr: TokenStream, item: TokenStream) -> TokenStream {
    integration::context_mapping(attr, item)
}

/// Marks a struct as an ESDM `process-manager` scoped to a `Domain`.
///
/// # Attributes
/// - `scope = <Domain>` *(required)*: The domain this process manager belongs to.
/// - `correlated_by = event_field(<field>) | event_subject` *(required)*: How incoming events
///   are correlated to a process instance.
/// - `starts_when = [<Event>, ...]` *(required)*: Events that may instantiate a new process
///   instance.
/// - `handles = [<Event>, ...]` *(optional)*: Additional events handled by an active
///   process instance.
/// - `emits = [<Command>, ...]` *(optional when `timers` is provided, otherwise required)*:
///   Commands this process manager may emit.
/// - `timers = [<Timer>(after(<unit> = <value>)) | <Timer>(at = <field>), ...]` *(optional)*:
///   Timers managed by this process manager.
/// - `ends_when = <predicate>` *(required)*: Closure or function `fn(&Self) -> bool`
///   indicating when the process instance has reached a terminal state.
/// - `delivery_guarantee = at_least_once | at_most_once` *(optional)*: Delivery guarantee
///   for incoming events (`at_least_once` requires `idempotency`).
/// - `idempotency = self | downstream | infrastructure | none | not_required` or
///   `idempotency(owner = ..., strategy = "...")` *(required when `delivery_guarantee = at_least_once`)*.
#[proc_macro_attribute]
pub fn process_manager(attr: TokenStream, item: TokenStream) -> TokenStream {
    integration::process_manager(attr, item)
}

/// Derives `chromatofoor::architecturekit::Schema` for structs and unit-variant enums
/// without requiring a direct `architecturekit` dependency in downstream crates.
#[proc_macro_derive(Schema)]
pub fn derive_schema(item: TokenStream) -> TokenStream {
    schema::derive_schema(item)
}

/// Derives `chromatofoor::architecturekit::Events` for event sum-type enums
/// without requiring a direct `architecturekit` dependency in downstream crates.
#[proc_macro_derive(Events)]
pub fn derive_events(item: TokenStream) -> TokenStream {
    schema::derive_events(item)
}
