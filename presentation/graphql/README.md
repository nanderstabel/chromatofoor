# Chromatofoor GraphQL Presentation Adapter (`chromatofoor-graphql`)

## Overview
`chromatofoor-graphql` builds a dynamic `async-graphql` (`~7.0.0`, `dynamic-schema`) schema at runtime from a `chromatofoor::domain::DomainRuntime`. It synthesizes typed GraphQL `Query`, `Mutation`, and live `Subscription` fields, nested `InputObject` and `Object` types, a fallback `JSON` scalar, and schema-driven `camelCase` <-> `snake_case` field key mapping—either executed in-process or mounted onto `chromatofoor-http` as an `HttpProtocol` sub-router serving GraphiQL, HTTP `GET`/`POST`, SSE, and WebSockets.

## Interface
- **Cargo Feature Flags** (`Cargo.toml`):
  - `default = []` — In-process dynamic schema construction and execution (`build_schema`, `execute_as`, `type_mapper`) with zero HTTP dependencies.
  - `http` — Enables `dep:axum`, `dep:async-graphql-axum`, `dep:chromatofoor-http`, and `dep:tokio-stream` (`GraphQL`, `routes`).
  - `axum` — Alias enabling `http`.
- **Dynamic Schema & In-Process Execution** (`src/lib.rs`):
  - `build_schema(runtime: Arc<DomainRuntime>) -> Result<async_graphql::dynamic::Schema, async_graphql::dynamic::SchemaError>` — Synthesizes `Query` (`ping` + domain queries), `Mutation` (domain commands), `Subscription` (`events`), `JSON` scalar, and `DomainEvent` object (`id`, `subject`, `ty`, `data`).
  - `async fn execute_as(schema: &async_graphql::dynamic::Schema, query: &str, actor: Option<&str>) -> async_graphql::Response` — Executes a GraphQL operation against `schema` injecting optional `{"actor_type": actor}` context data.
- **Composable HTTP / SSE / WebSocket Adapter** (`src/lib.rs`, `#[cfg(feature = "http")]`):
  - `GraphQL` — Zero-sized adapter (`Debug`, `Default`, `Clone`, `Copy`, `pub const fn new() -> Self`) implementing `chromatofoor_http::HttpProtocol` (`name() -> "graphql"`, `router` delegating to `routes`; does not implement standalone `Presentation`).
  - `routes(runtime: Arc<DomainRuntime>) -> axum::Router` — Mounts:
    - `GET /` & `GET /graphiql` — GraphiQL IDE HTML (`endpoint: "/graphql"`, `subscription_endpoint: "/graphql/ws"`).
    - `GET /graphql` & `POST /graphql` — GraphQL execution supporting `query`, `operationName` (`operation_name`), and `variables` (falling back to GraphiQL on `GET /graphql` without `?query=`, and switching to SSE when `Accept` contains `text/event-stream` without `application/json` or the query begins with `subscription`).
    - `GET /graphql/sse` & `POST /graphql/sse` — GraphQL over SSE (`next` data frames followed by a `complete` frame; `GET` without `?query=` returns `400 Bad Request`).
    - `/graphql/ws` — WebSocket subscription service (`async_graphql_axum::GraphQLSubscription`).
- **JSON Schema Type Mapper** (`src/type_mapper.rs`, `pub mod type_mapper`):
  - `OutputShape` (`Scalar`, `Object`, `ObjectList`), `json_to_field_value`.
  - `is_nullable`, `resolve_schema`, `effective_type`, `build_type_ref`.
  - `create_input_objects`, `create_input_object`, `build_query_output_type`, `create_object`.

## Invariants
- **Safety, Lint Baseline & Non-Empty Command Precondition** (`Cargo.toml`, `src/lib.rs`):
  - Inherits workspace `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)` (only `routes` carries `#[allow(clippy::expect_used)]`).
  - Because `build_schema` unconditionally registers `Object::new("Mutation")` (while seeding `Query` with `ping: String!` -> `"pong"`), `DomainRuntime` must register at least one command—otherwise `build_schema` returns `Err(SchemaError)` and `routes` panics.
- **Field Naming & Schema-Indexed Case Mapping** (`src/lib.rs:build_schema`, `collect_key_mappings`, `map_json_keys`):
  - Commands map to `camelCase` `Mutation` fields (`{boundedContext}{Target}{Name}`, returning `String!` `"Success"`) with required `payload: <...>Input!` when `schema["properties"]` is non-empty (omitted when empty) and optional `actor: String`.
  - Queries map to `camelCase` `Query` fields (`{boundedContext}{Name}`) with required `params: <...>ParamsInput!` when `parameters_schema["properties"]` is non-empty (omitted when empty) and optional `actor: String`.
  - All schema property names are indexed in a global `HashMap<camel_key, schema_key>` used to rewrite incoming `payload`/`params` JSON keys prior to `DomainRuntime` dispatch, while generated output `Object` resolvers check `parent[snake_key]` then `parent[camel_key]`.
- **Actor Precedence** (`src/lib.rs:build_schema`, `attach_header_metadata`): Explicit GraphQL field argument `actor` takes precedence over request context `"actor_type"`, which takes precedence over `"actor"`. On HTTP `GET`/`POST`, `attach_header_metadata` seeds `"actor_type"` via `chromatofoor_http::actor_from_headers` and then applies normalized `x-metadata-*` headers (so `x-metadata-actor-type` overrides `actor_from_headers`); `/graphql/ws` uses `GraphQLSubscription` directly without HTTP header metadata extraction.
- **Subscription Filtering & Bounded Schema Recursion** (`src/lib.rs:build_schema`, `src/type_mapper.rs`):
  - `Subscription.events(subjectPrefix: String, eventType: String)` filters `subject` by prefix (`starts_with`) and `ty` by substring (`contains`).
  - `$ref` resolution (`#/$defs/*`, `#/definitions/*`), single-element `allOf`, and `anyOf`/`oneOf` unwrapping halt at `depth > 8`, while nested `InputObject`/`Object` synthesis halts at `depth >= 8` (falling back to the `JSON` scalar, which is also used for property-less dynamic map objects); `OutputShape::ObjectList` types items as `NonNull` (`[...ItemOutput!]`) and filters out `null` elements.

## Side Effects
- **Store Mutations, Query Reads & Event Streams** (`src/lib.rs:build_schema`, `execute_as`): Mutation resolvers invoke `DomainRuntime::dispatch_command`, Query resolvers invoke `DomainRuntime::dispatch_query`, and `Subscription.events` subscribes to `DomainRuntime::subscribe_events()`.
- **HTTP, SSE & WebSocket I/O** (`src/lib.rs:routes`, feature `http`): Serves GraphiQL HTML, JSON responses, SSE event streams (`next` + `complete`), and WebSocket GraphQL subscriptions via Axum.

## Verification
```bash
cargo test -p chromatofoor-graphql --all-features
cargo clippy -p chromatofoor-graphql --all-targets --all-features -- -D warnings
```
