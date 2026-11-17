# Chromatofoor HTTP Presentation Adapter (`chromatofoor-http`)

## Overview
`chromatofoor-http` provides the composable Axum 0.8 HTTP (`GET`, `POST`, `SSE`) and OpenAPI 3.0 presentation server for `chromatofoor::domain::DomainRuntime`. It exposes `HttpServer` (implementing `chromatofoor::runtime::Presentation`) alongside the `HttpProtocol` sub-router trait so additional HTTP protocols (such as `chromatofoor-graphql` and `chromatofoor-mcp` Streamable HTTP) can share a single TCP listener and `DomainRuntime` without built-in CORS assumptions.

## Interface
- **Sub-Protocol Extension Trait** (`src/lib.rs` — no Cargo feature flags):
  - `HttpProtocol` — `Send + 'static` trait (`fn name(&self) -> &'static str` defaulted via `type_name::<Self>()`, and required `fn router(self, runtime: Arc<DomainRuntime>) -> axum::Router`), blanket-implemented for `FnOnce(Arc<DomainRuntime>) -> axum::Router + Send + 'static`.
- **Composable HTTP Server & Bind Targets** (`src/lib.rs`):
  - `BindTarget` — `Socket(SocketAddr)` | `Address(String)` (defaults to `0.0.0.0:3000`; implements `Display` and `From<SocketAddr | ([u8; 4], u16) | u16 | &str | String>`).
  - `HttpServer` — Builder implementing `Default` (`0.0.0.0:3000`) and `chromatofoor::runtime::Presentation` (`name() -> "http"`, `async fn serve`):
    - `HttpServer::bind(target: impl Into<BindTarget>) -> Self` (and alias `HttpServer::new`).
    - `without_default_routes(self) -> Self` — Disables built-in `/openapi.json` and `/v0/...` routes so only attached `HttpProtocol` routers are mounted.
    - `attach<P: HttpProtocol>(self, protocol: P) -> Self` — Registers an `HttpProtocol` sub-router in order.
    - `attached_protocols(&self) -> Vec<&'static str>`, `bind_target(&self) -> &BindTarget`.
    - `into_router(self, runtime: Arc<DomainRuntime>) -> axum::Router` — Builds and merges the default and attached `HttpProtocol` routers (no built-in CORS middleware).
- **Standalone Router, OpenAPI & Header Helpers** (`src/lib.rs`):
  - `routes(runtime: Arc<DomainRuntime>) -> axum::Router` — Mounts:
    - `GET /openapi.json` — OpenAPI 3.0 JSON specification (`200 OK`).
    - `GET /v0/catalog` — Runtime domain catalog JSON (`200 OK`).
    - `GET /v0/events` — Live/replayed SSE event stream (`text/event-stream`, filtered by optional `?subject_prefix=` prefix and `?event_type=` substring query parameters).
    - `POST /v0/{bc}/{target}/{cmd}` & `POST /v0/{bc}/{target}/{cmd}/{id}` — Command dispatch returning emitted event count (`usize` JSON, `200 OK`).
    - `GET /v0/{bc}/{query}`, `GET /v0/{bc}/{query}/{id}`, `GET /v0/{bc}/views/{read_model}/{query}`, & `GET /v0/{bc}/views/{read_model}/{query}/{id}` — Read-model query dispatch (`200 OK`).
  - `generate_openapi(runtime: &DomainRuntime) -> utoipa::openapi::OpenApi` — Generates the `{domain_name} HTTP API` (`version: "1.0"`) OpenAPI 3.0 specification for registered command `POST` and query `GET` routes.
  - `actor_from_headers(headers: &HeaderMap) -> Option<&str>` — Extracts the caller actor string from HTTP request headers.

## Invariants
- **Strict Rust Safety & Lint Baseline** (`Cargo.toml`): Inherits workspace lints enforcing `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)`.
- **Actor Header Precedence** (`src/lib.rs:actor_from_headers`): Evaluates headers in strict priority order `x-actor-type` > `x-actor` > `x-metadata-actor-type` > `x-metadata-actor` before UTF-8 decoding (invalid UTF-8 on a higher-priority header yields `None` without falling back).
- **Path `{id}` Injection & Body/Query Precedence** (`src/lib.rs:inject_command_id`, `src/lib.rs:inject_query_id`, `src/lib.rs:coerce_query_params`):
  - On `POST .../{cmd}/{id}`, path `{id}` populates unset JSON object keys matching `IdSource::State(f) | CommandPayload(f) | Schema(f)` (or `"id"` if none match), never overwriting explicit JSON body fields.
  - On `GET .../{query}/{id}`, query-string parameters are coerced first (`integer` -> `i64`, `number` -> finite `f64` or `null`, `boolean` -> `bool`, fallback `String`), then path `{id}` is injected as `Value::String` without overwriting existing keys: if `"id"` is declared in `schema.properties`, `{id}` binds to `"id"` (if unset) and returns; otherwise it binds to the first unset `*_id` property -> first unset `required` property -> first unset property (returning as a no-op if all declared properties are already set), or falls back to `"id"` when `schema.properties` is absent/empty.
- **Bounded Schema `$ref` Resolution** (`src/lib.rs:resolve_schema_node`): Recursive `$ref` (`#/$defs/` or `#/definitions/`), single-element `allOf`, and nullable `anyOf`/`oneOf` resolution halts at depth `> 8` (falling back to an empty `Type::Object` in OpenAPI) to prevent infinite recursion on cyclic schemas.
- **RFC 9457 Problem Details Error Mapping & Router Disjointness** (`src/lib.rs`): `DispatchError` serializes via `DispatchError::to_problem()` as `application/problem+json` (referencing `#/components/schemas/ProblemDetails` in OpenAPI 3.0), mapping missing bounded contexts/commands/queries/aggregate streams/read-model records to `404 Not Found`, unauthorized actors to `403 Forbidden`, malformed payloads to `400 Bad Request`, stream lifecycle/concurrency conflicts to `409 Conflict`, and domain invariant/validation violations to `422 Unprocessable Entity`. Attached `HttpProtocol` routes must not collide with enabled default routes or sibling protocols (Axum `Router::merge` panics on duplicate route paths), and CORS middleware must be layered externally when needed.

## Side Effects
- **TCP Listener & HTTP Serving** (`src/lib.rs:HttpServer::serve`): Binds `tokio::net::TcpListener`, logs the listening target via `tracing::info!`, and serves HTTP requests via `axum::serve`.
- **Store Mutations, Query Reads & SSE Subscriptions** (`src/lib.rs:routes`): Appends domain events via `DomainRuntime::dispatch_command` (`POST`), reads projections via `DomainRuntime::dispatch_query` (`GET`), and subscribes to `DomainRuntime::subscribe_events()` (`UnboundedReceiverStream`) on `GET /v0/events` to stream SSE frames (`event`, `id`, JSON `data`).

## Verification
```bash
cargo test -p chromatofoor-http --all-features
cargo clippy -p chromatofoor-http --all-targets --all-features -- -D warnings
```
