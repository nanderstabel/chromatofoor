//! HTTP (`GET`, `POST`, `SSE`) and OpenAPI presentation foundation for
//! [`chromatofoor::domain::DomainRuntime`].
//!
//! Automatically mounts:
//! - `GET /openapi.json` — OpenAPI 3.0 specification derived from registered commands and queries.
//! - `GET /v0/catalog` — JSON domain catalog of actors, commands, queries, read models, and consumers.
//! - `GET /v0/events` — Server-Sent Events (SSE) stream of all domain events.
//! - `POST /v0/{bounded_context}/{target}/{command}` and
//!   `POST /v0/{bounded_context}/{target}/{command}/{id}` — Command endpoints.
//! - `GET /v0/{bounded_context}/{query}` and `GET /v0/{bounded_context}/{query}/{id}` — Query
//!   endpoints.
//!
//! Higher-level HTTP protocols (such as `chromatofoor_graphql::GraphQL`,
//! `chromatofoor_mcp::Mcp::http()`, or custom third-party adapters) implement [`HttpProtocol`] and
//! attach to [`HttpServer`] via [`HttpServer::attach`], sharing a single HTTP listener and
//! [`DomainRuntime`].
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use axum::{
//!     body::Body,
//!     http::{Request, StatusCode},
//! };
//! use chromatofoor::{
//!     architecturekit::{
//!         self, Envelope, InMemoryEventStore, InMemoryView, KeyedView, Store, ViewError,
//!     },
//!     bounded_context::{Command, Event, Project, Query},
//!     domain::DomainBuilder,
//! };
//! use tower::ServiceExt as _;
//!
//! #[chromatofoor::domain]
//! pub struct Commerce;
//!
//! #[chromatofoor::bounded_context(scope = Commerce)]
//! pub struct Ordering;
//!
//! #[chromatofoor::actor(scope = Ordering, type = human)]
//! pub struct Customer;
//!
//! #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
//! #[derive(Clone, Default)]
//! pub struct Order {
//!     pub order_id: Option<chromatofoor::Identifier>,
//!     pub total_cents: u64,
//! }
//!
//! #[chromatofoor::event(scope = Order)]
//! pub enum OrderEvent {
//!     OrderPlaced { order_id: chromatofoor::Identifier, total_cents: u64 },
//! }
//!
//! impl Event<Order> for OrderPlaced {
//!     fn evolve(&self, _state: Order) -> Order {
//!         Order {
//!             order_id: Some(self.order_id.clone()),
//!             total_cents: self.total_cents,
//!         }
//!     }
//! }
//!
//! #[chromatofoor::command(scope = Order)]
//! pub enum OrderCommand {
//!     /// Places a new customer order.
//!     #[lifecycle(create)]
//!     #[actors(Customer)]
//!     PlaceOrder { order_id: chromatofoor::Identifier, total_cents: u64 },
//! }
//!
//! impl Command<Order> for PlaceOrder {
//!     type Error = &'static str;
//!
//!     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
//!         Ok(vec![OrderPlaced {
//!             order_id: self.order_id,
//!             total_cents: self.total_cents,
//!         }
//!         .into()])
//!     }
//! }
//!
//! #[chromatofoor::read_model(scope = Ordering, events = [OrderPlaced])]
//! pub struct OrderTotals {
//!     pub totals: InMemoryView<String, (String, u64)>,
//! }
//!
//! impl Project<OrderPlaced> for OrderTotals {
//!     async fn project(&self, event: Envelope<OrderPlaced>) -> Result<(), ViewError> {
//!         self.totals
//!             .insert(&event, (event.data.order_id.to_string(), event.data.total_cents))?;
//!         Ok(())
//!     }
//! }
//!
//! #[chromatofoor::query(scope = Ordering, actors = [Customer])]
//! pub struct OrderTotalById {
//!     pub order_id: String,
//! }
//!
//! impl Query<OrderTotals> for OrderTotalById {
//!     type Output = Option<u64>;
//!     type Error = std::convert::Infallible;
//!
//!     async fn execute(self, read_model: &OrderTotals) -> Result<Self::Output, Self::Error> {
//!         let entry = read_model.totals.get(&self.order_id).await?;
//!         Ok(entry.map(|(_, cents)| cents))
//!     }
//! }
//!
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! let store = Arc::new(
//!     Store::builder(InMemoryEventStore::new(), "https://commerce.local").build(),
//! );
//! let order_totals = Arc::new(OrderTotals {
//!     totals: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
//! });
//!
//! let runtime = Arc::new(
//!     DomainBuilder::<Commerce>::new(store)
//!         .register_command::<PlaceOrder, _>()
//!         .register_read_model(Arc::clone(&order_totals))
//!         .register_query::<OrderTotalById, _>(Arc::clone(&order_totals))
//!         .build()
//!         .await
//!         .unwrap(),
//! );
//!
//! let app = chromatofoor_http::HttpServer::bind("0.0.0.0:3000").into_router(Arc::clone(&runtime));
//!
//! // 1. Dispatch PlaceOrder via POST /v0/ordering/order/place-order/ord-1:
//! let req = Request::builder()
//!     .method("POST")
//!     .uri("/v0/ordering/order/place-order/ord-1")
//!     .header("content-type", "application/json")
//!     .header("x-actor-type", "customer")
//!     .body(Body::from(r#"{"total_cents": 4200}"#))
//!     .unwrap();
//! let resp = app.clone().oneshot(req).await.unwrap();
//! assert_eq!(resp.status(), StatusCode::OK);
//!
//! // 2. Query OrderTotalById via GET /v0/ordering/order-total-by-id/ord-1:
//! runtime.wait_caught_up().await.unwrap();
//! let req = Request::builder()
//!     .method("GET")
//!     .uri("/v0/ordering/order-total-by-id/ord-1")
//!     .header("x-actor-type", "customer")
//!     .body(Body::empty())
//!     .unwrap();
//! let resp = app.oneshot(req).await.unwrap();
//! assert_eq!(resp.status(), StatusCode::OK);
//! # });
//! ```

use std::{collections::HashMap, convert::Infallible, net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query as AxumQuery, State},
    http::{HeaderMap, StatusCode},
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use chromatofoor::{
    common::IdSource,
    domain::DomainRuntime,
    runtime::{CommandDescriptor, DispatchError, Presentation},
};
use serde_json::Value;
use tokio_stream::StreamExt as _;
use utoipa::openapi::{
    ContentBuilder, HttpMethod, InfoBuilder, OpenApi, OpenApiBuilder, PathItem, PathsBuilder,
    path::OperationBuilder,
    request_body::RequestBodyBuilder,
    schema::{ArrayBuilder, ObjectBuilder, Schema, SchemaType, Type},
};

/// An HTTP-based protocol adapter (such as `chromatofoor_graphql::GraphQL`,
/// `chromatofoor_mcp::Mcp::http()`, or a custom third-party adapter) that can be attached to an
/// [`HttpServer`] via [`HttpServer::attach`].
pub trait HttpProtocol: Send + 'static {
    /// Identifier for this HTTP protocol adapter (e.g. `"graphql"`, `"mcp-http"`).
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Builds an [`axum::Router`] for this protocol backed by `runtime`.
    fn router(self, runtime: Arc<DomainRuntime>) -> Router;
}

impl<F> HttpProtocol for F
where
    F: FnOnce(Arc<DomainRuntime>) -> Router + Send + 'static,
{
    fn router(self, runtime: Arc<DomainRuntime>) -> Router {
        self(runtime)
    }
}

trait ErasedHttpProtocol: Send {
    fn name(&self) -> &'static str;
    fn into_router(self: Box<Self>, runtime: Arc<DomainRuntime>) -> Router;
}

impl<P: HttpProtocol> ErasedHttpProtocol for P {
    fn name(&self) -> &'static str {
        HttpProtocol::name(self)
    }

    fn into_router(self: Box<Self>, runtime: Arc<DomainRuntime>) -> Router {
        (*self).router(runtime)
    }
}

/// Network address or socket specification that [`HttpServer`] binds to when serving.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindTarget {
    /// A resolved [`SocketAddr`].
    Socket(SocketAddr),
    /// A host/port string (e.g. `"0.0.0.0:3000"` or `"127.0.0.1:8080"`).
    Address(String),
}

impl Default for BindTarget {
    fn default() -> Self {
        Self::Socket(SocketAddr::from(([0, 0, 0, 0], 3000)))
    }
}

impl std::fmt::Display for BindTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Socket(addr) => write!(f, "{addr}"),
            Self::Address(s) => write!(f, "{s}"),
        }
    }
}

impl From<SocketAddr> for BindTarget {
    fn from(addr: SocketAddr) -> Self {
        Self::Socket(addr)
    }
}

impl From<([u8; 4], u16)> for BindTarget {
    fn from(value: ([u8; 4], u16)) -> Self {
        Self::Socket(SocketAddr::from(value))
    }
}

impl From<u16> for BindTarget {
    fn from(port: u16) -> Self {
        Self::Socket(SocketAddr::from(([0, 0, 0, 0], port)))
    }
}

impl From<&str> for BindTarget {
    fn from(value: &str) -> Self {
        Self::Address(value.to_owned())
    }
}

impl From<String> for BindTarget {
    fn from(value: String) -> Self {
        Self::Address(value)
    }
}

/// Composable HTTP presentation server implementing [`chromatofoor::runtime::Presentation`].
///
/// By default, [`HttpServer`] mounts the core HTTP (`GET`, `POST`, `SSE`) and OpenAPI routes
/// from [`routes`], and allows additional [`HttpProtocol`] adapters (such as GraphQL or MCP
/// Streamable HTTP) to be attached onto the same listener via [`Self::attach`].
pub struct HttpServer {
    target: BindTarget,
    include_default_routes: bool,
    protocols: Vec<Box<dyn ErasedHttpProtocol>>,
}

impl Default for HttpServer {
    fn default() -> Self {
        Self {
            target: BindTarget::default(),
            include_default_routes: true,
            protocols: Vec::new(),
        }
    }
}

impl HttpServer {
    /// Creates a new [`HttpServer`] configured to bind to `target` (e.g. `"0.0.0.0:3000"`,
    /// `([0, 0, 0, 0], 3000)`, or a `u16` port).
    #[must_use]
    pub fn bind(target: impl Into<BindTarget>) -> Self {
        Self {
            target: target.into(),
            include_default_routes: true,
            protocols: Vec::new(),
        }
    }

    /// Alias for [`Self::bind`].
    #[must_use]
    pub fn new(target: impl Into<BindTarget>) -> Self {
        Self::bind(target)
    }

    /// Disables the default `/v0/...` and `/openapi.json` routes so only explicitly attached
    /// [`HttpProtocol`]s are served on this listener.
    #[must_use]
    pub fn without_default_routes(mut self) -> Self {
        self.include_default_routes = false;
        self
    }

    /// Attaches an [`HttpProtocol`] sub-adapter (such as `chromatofoor_graphql::GraphQL` or
    /// `chromatofoor_mcp::Mcp::http()`) to share this HTTP server's listener and [`DomainRuntime`].
    #[must_use]
    pub fn attach<P: HttpProtocol>(mut self, protocol: P) -> Self {
        self.protocols.push(Box::new(protocol));
        self
    }

    /// Returns the names of all [`HttpProtocol`]s attached to this server.
    #[must_use]
    pub fn attached_protocols(&self) -> Vec<&'static str> {
        self.protocols.iter().map(|p| p.name()).collect()
    }

    /// Returns the configured [`BindTarget`].
    #[must_use]
    pub fn bind_target(&self) -> &BindTarget {
        &self.target
    }

    /// Builds the combined [`axum::Router`] merging the default HTTP routes (unless disabled via
    /// [`Self::without_default_routes`]) and all attached [`HttpProtocol`] routers.
    pub fn into_router(self, runtime: Arc<DomainRuntime>) -> Router {
        let mut router = if self.include_default_routes {
            routes(Arc::clone(&runtime))
        } else {
            Router::new()
        };

        for protocol in self.protocols {
            router = router.merge(protocol.into_router(Arc::clone(&runtime)));
        }

        router
    }
}

impl Presentation for HttpServer {
    fn name(&self) -> &'static str {
        "http"
    }

    async fn serve(
        self,
        runtime: Arc<DomainRuntime>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let target = self.target.clone();
        let app = self.into_router(runtime);
        let listener = match &target {
            BindTarget::Socket(addr) => tokio::net::TcpListener::bind(addr).await?,
            BindTarget::Address(addr) => tokio::net::TcpListener::bind(addr.as_str()).await?,
        };
        tracing::info!("HTTP server listening on {target}");
        axum::serve(listener, app).await?;
        Ok(())
    }
}

fn resolve_schema_node<'a>(node: &'a Value, root: &'a Value, depth: usize) -> &'a Value {
    if depth > 8 {
        return node;
    }
    if let Some(ref_str) = node.get("$ref").and_then(Value::as_str) {
        let def_name = ref_str
            .strip_prefix("#/$defs/")
            .or_else(|| ref_str.strip_prefix("#/definitions/"));
        if let Some(name) = def_name
            && let Some(resolved) = root
                .get("$defs")
                .or_else(|| root.get("definitions"))
                .and_then(|defs| defs.get(name))
        {
            return resolve_schema_node(resolved, root, depth + 1);
        }
    }
    if let Some(all_of) = node.get("allOf").and_then(Value::as_array)
        && all_of.len() == 1
    {
        return resolve_schema_node(&all_of[0], root, depth + 1);
    }
    if let Some(branches) = node
        .get("anyOf")
        .or_else(|| node.get("oneOf"))
        .and_then(Value::as_array)
        && let Some(non_null) = branches
            .iter()
            .find(|b| b.get("type").and_then(Value::as_str) != Some("null"))
    {
        return resolve_schema_node(non_null, root, depth + 1);
    }
    node
}

fn effective_schema_type(node: &Value) -> &str {
    if let Some(ty) = node.get("type") {
        if let Some(s) = ty.as_str() {
            return s;
        }
        if let Some(arr) = ty.as_array()
            && let Some(non_null) = arr.iter().filter_map(Value::as_str).find(|s| *s != "null")
        {
            return non_null;
        }
    }
    if node.get("properties").is_some() {
        return "object";
    }
    "string"
}

fn json_schema_node_to_openapi(raw_node: &Value, root_schema: &Value, depth: usize) -> Schema {
    if depth > 8 {
        return ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::Object))
            .build()
            .into();
    }
    let node = resolve_schema_node(raw_node, root_schema, depth);
    match effective_schema_type(node) {
        "integer" => ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::Integer))
            .build()
            .into(),
        "number" => ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::Number))
            .build()
            .into(),
        "boolean" => ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::Boolean))
            .build()
            .into(),
        "array" => {
            let item_schema = node.get("items").map_or_else(
                || {
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build()
                        .into()
                },
                |items| json_schema_node_to_openapi(items, root_schema, depth + 1),
            );
            ArrayBuilder::new()
                .items(utoipa::openapi::RefOr::T(item_schema))
                .build()
                .into()
        }
        "object" => {
            let mut builder = ObjectBuilder::new().schema_type(SchemaType::Type(Type::Object));
            if let Some(required) = node.get("required").and_then(Value::as_array) {
                for req in required.iter().filter_map(Value::as_str) {
                    builder = builder.required(req.to_owned());
                }
            }
            if let Some(props) = node.get("properties").and_then(Value::as_object) {
                for (prop_name, prop_schema) in props {
                    let child = json_schema_node_to_openapi(prop_schema, root_schema, depth + 1);
                    builder = builder.property(prop_name.clone(), child);
                }
            }
            builder.build().into()
        }
        _ => ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::String))
            .build()
            .into(),
    }
}

fn json_schema_to_openapi(schema: &Value) -> Schema {
    json_schema_node_to_openapi(schema, schema, 0)
}

/// Extracts the caller's actor role from `X-Actor-Type`, `X-Actor`, `X-Metadata-Actor-Type`,
/// or `X-Metadata-Actor` HTTP request headers.
#[must_use]
pub fn actor_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-actor-type")
        .or_else(|| headers.get("x-actor"))
        .or_else(|| headers.get("x-metadata-actor-type"))
        .or_else(|| headers.get("x-metadata-actor"))
        .and_then(|v| v.to_str().ok())
}

fn inject_command_id(payload: &mut Value, cmd: &CommandDescriptor, id: &str) {
    let Some(obj) = payload.as_object_mut() else {
        return;
    };
    let mut injected = false;
    for source in cmd.identified_by {
        if let IdSource::State(field) | IdSource::CommandPayload(field) | IdSource::Schema(field) =
            source
        {
            obj.entry((*field).to_owned())
                .or_insert_with(|| Value::String(id.to_owned()));
            injected = true;
        }
    }
    if !injected {
        obj.entry("id".to_owned())
            .or_insert_with(|| Value::String(id.to_owned()));
    }
}

fn inject_query_id(params: &mut Value, query_schema: &Value, id: &str) {
    let Some(obj) = params.as_object_mut() else {
        return;
    };
    let resolved = resolve_schema_node(query_schema, query_schema, 0);
    if let Some(props) = resolved.get("properties").and_then(Value::as_object) {
        if props.contains_key("id") {
            obj.entry("id".to_owned())
                .or_insert_with(|| Value::String(id.to_owned()));
            return;
        }
        if let Some(id_key) = props
            .keys()
            .find(|k| k.ends_with("_id") && !obj.contains_key(*k))
            .or_else(|| {
                resolved
                    .get("required")
                    .and_then(Value::as_array)
                    .and_then(|req| {
                        req.iter()
                            .filter_map(Value::as_str)
                            .find(|k| !obj.contains_key(*k))
                    })
                    .and_then(|k| props.get_key_value(k).map(|(key, _)| key))
            })
            .or_else(|| props.keys().find(|k| !obj.contains_key(*k)))
            .or_else(|| props.keys().find(|k| k.ends_with("_id")))
            .or_else(|| props.keys().next())
        {
            obj.entry(id_key.clone())
                .or_insert_with(|| Value::String(id.to_owned()));
            return;
        }
    }
    obj.entry("id".to_owned())
        .or_insert_with(|| Value::String(id.to_owned()));
}

fn coerce_query_params(raw: HashMap<String, String>, query_schema: &Value) -> Value {
    let resolved = resolve_schema_node(query_schema, query_schema, 0);
    let props = resolved.get("properties").and_then(Value::as_object);
    let mut map = serde_json::Map::new();
    for (k, v) in raw {
        let field_type = props
            .and_then(|p| p.get(&k))
            .map(|s| resolve_schema_node(s, query_schema, 0))
            .map(effective_schema_type);
        let coerced = match field_type {
            Some("integer") => v
                .parse::<i64>()
                .map_or_else(|_| Value::String(v), Value::from),
            Some("number") => v
                .parse::<f64>()
                .map_or_else(|_| Value::String(v), Value::from),
            Some("boolean") => v
                .parse::<bool>()
                .map_or_else(|_| Value::String(v), Value::from),
            _ => Value::String(v),
        };
        map.insert(k, coerced);
    }
    Value::Object(map)
}

fn map_dispatch_error(err: DispatchError) -> axum::response::Response {
    let status = match &err {
        DispatchError::BoundedContextNotFound(_)
        | DispatchError::CommandNotFound { .. }
        | DispatchError::QueryNotFound { .. } => StatusCode::NOT_FOUND,
        DispatchError::Unauthorized { .. } => StatusCode::FORBIDDEN,
        DispatchError::InvalidPayload(_) => StatusCode::BAD_REQUEST,
        DispatchError::Execution(_) => StatusCode::UNPROCESSABLE_ENTITY,
    };
    (status, err.to_string()).into_response()
}

/// Generates an OpenAPI 3.0 specification for all commands and queries registered in `runtime`.
#[must_use]
pub fn generate_openapi(runtime: &DomainRuntime) -> OpenApi {
    let mut paths = PathsBuilder::new();

    for cmd in runtime.commands() {
        let base_path = format!("/v0/{}/{}/{}", cmd.bounded_context, cmd.target, cmd.name);
        let id_path = format!("{base_path}/{{id}}");

        let operation = OperationBuilder::new()
            .description(Some(cmd.description.to_owned()))
            .tag(cmd.target.to_owned())
            .request_body(Some(
                RequestBodyBuilder::new()
                    .content(
                        "application/json",
                        ContentBuilder::new()
                            .schema(Some(json_schema_to_openapi(&cmd.schema)))
                            .build(),
                    )
                    .build(),
            ))
            .build();

        paths = paths
            .path(
                base_path,
                PathItem::new(HttpMethod::Post, operation.clone()),
            )
            .path(id_path, PathItem::new(HttpMethod::Post, operation));
    }

    for q in runtime.queries() {
        let base_path = format!("/v0/{}/{}", q.bounded_context, q.name);
        let id_path = format!("{base_path}/{{id}}");
        let view_base_path = format!(
            "/v0/{}/views/{}/{}",
            q.bounded_context, q.read_model, q.name
        );
        let view_id_path = format!("{view_base_path}/{{id}}");

        let operation = OperationBuilder::new()
            .description(Some(q.description.to_owned()))
            .tag(q.bounded_context.to_owned())
            .build();

        paths = paths
            .path(base_path, PathItem::new(HttpMethod::Get, operation.clone()))
            .path(id_path, PathItem::new(HttpMethod::Get, operation.clone()))
            .path(
                view_base_path,
                PathItem::new(HttpMethod::Get, operation.clone()),
            )
            .path(view_id_path, PathItem::new(HttpMethod::Get, operation));
    }

    OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title(format!("{} HTTP API", runtime.domain_name()))
                .description(Some(runtime.domain_description().to_owned()))
                .version("1.0")
                .build(),
        )
        .paths(paths.build())
        .build()
}

/// Builds an [`axum::Router`] serving OpenAPI, domain catalog, SSE events, commands, and queries
/// for `runtime`.
pub fn routes(runtime: Arc<DomainRuntime>) -> Router {
    let openapi_spec = generate_openapi(&runtime);
    let mut router = Router::new()
        .route(
            "/openapi.json",
            get(move || async move { Json(openapi_spec) }),
        )
        .route(
            "/v0/catalog",
            get(
                |State(state): State<Arc<DomainRuntime>>| async move { Json(state.catalog_json()) },
            ),
        )
        .route(
            "/v0/events",
            get(
                |State(state): State<Arc<DomainRuntime>>,
                 AxumQuery(filters): AxumQuery<HashMap<String, String>>| async move {
                    let subject_prefix = filters.get("subject_prefix").cloned();
                    let event_type = filters.get("event_type").cloned();
                    let rx = state.subscribe_events();
                    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
                        .filter_map(move |event| {
                            if let Some(ref prefix) = subject_prefix
                                && !event.subject().starts_with(prefix)
                            {
                                return None;
                            }
                            if let Some(ref ty) = event_type
                                && !event.ty().contains(ty)
                            {
                                return None;
                            }
                            let mut sse_event = Event::default().event(event.ty()).id(event.id());
                            if let Ok(json_str) = serde_json::to_string(event.data()) {
                                sse_event = sse_event.data(json_str);
                            }
                            Some(Ok::<_, Infallible>(sse_event))
                        });
                    Sse::new(stream).into_response()
                },
            ),
        );

    for cmd in runtime.commands() {
        let cmd_no_id = cmd.clone();
        let base_path = format!("/v0/{}/{}/{}", cmd.bounded_context, cmd.target, cmd.name);
        router = router.route(
            &base_path,
            post(
                move |State(state): State<Arc<DomainRuntime>>,
                      headers: HeaderMap,
                      Json(payload): Json<Value>| {
                    let cmd = cmd_no_id.clone();
                    async move {
                        let actor = actor_from_headers(&headers);
                        match state
                            .dispatch_command(
                                cmd.bounded_context,
                                cmd.target,
                                cmd.name,
                                payload,
                                actor,
                            )
                            .await
                        {
                            Ok(events) => Json(events.len()).into_response(),
                            Err(e) => map_dispatch_error(e),
                        }
                    }
                },
            ),
        );

        let cmd_with_id = cmd.clone();
        let id_path = format!("{base_path}/{{id}}");
        router = router.route(
            &id_path,
            post(
                move |State(state): State<Arc<DomainRuntime>>,
                      Path(id): Path<String>,
                      headers: HeaderMap,
                      Json(mut payload): Json<Value>| {
                    let cmd = cmd_with_id.clone();
                    async move {
                        inject_command_id(&mut payload, &cmd, &id);
                        let actor = actor_from_headers(&headers);
                        match state
                            .dispatch_command(
                                cmd.bounded_context,
                                cmd.target,
                                cmd.name,
                                payload,
                                actor,
                            )
                            .await
                        {
                            Ok(events) => Json(events.len()).into_response(),
                            Err(e) => map_dispatch_error(e),
                        }
                    }
                },
            ),
        );
    }

    for q in runtime.queries() {
        let base_paths = [
            format!("/v0/{}/{}", q.bounded_context, q.name),
            format!(
                "/v0/{}/views/{}/{}",
                q.bounded_context, q.read_model, q.name
            ),
        ];
        for base_path in base_paths {
            let q_no_id = q.clone();
            router = router.route(
                &base_path,
                get(
                    move |State(state): State<Arc<DomainRuntime>>,
                          AxumQuery(raw_params): AxumQuery<HashMap<String, String>>,
                          headers: HeaderMap| {
                        let q = q_no_id.clone();
                        async move {
                            let params = coerce_query_params(raw_params, &q.parameters_schema);
                            let actor = actor_from_headers(&headers);
                            match state
                                .dispatch_query(q.bounded_context, q.name, params, actor)
                                .await
                            {
                                Ok(val) => Json(val).into_response(),
                                Err(e) => map_dispatch_error(e),
                            }
                        }
                    },
                ),
            );

            let q_with_id = q.clone();
            let id_path = format!("{base_path}/{{id}}");
            router = router.route(
                &id_path,
                get(
                    move |State(state): State<Arc<DomainRuntime>>,
                          Path(id): Path<String>,
                          AxumQuery(raw_params): AxumQuery<HashMap<String, String>>,
                          headers: HeaderMap| {
                        let q = q_with_id.clone();
                        async move {
                            let mut params = coerce_query_params(raw_params, &q.parameters_schema);
                            inject_query_id(&mut params, &q.parameters_schema, &id);
                            let actor = actor_from_headers(&headers);
                            match state
                                .dispatch_query(q.bounded_context, q.name, params, actor)
                                .await
                            {
                                Ok(val) => Json(val).into_response(),
                                Err(e) => map_dispatch_error(e),
                            }
                        }
                    },
                ),
            );
        }
    }

    router.with_state(runtime)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use chromatofoor::{
        architecturekit::{
            Envelope, InMemoryEventStore, InMemoryView, KeyedView, Store, ViewError,
        },
        bounded_context::{Command, Event, Project, Query},
        domain::DomainBuilder,
    };
    use http_body_util::BodyExt as _;
    use serde_json::{Value, json};
    use tower::ServiceExt as _;

    use super::*;

    #[chromatofoor::domain]
    pub struct DemoDomain;

    #[chromatofoor::bounded_context(scope = DemoDomain)]
    pub struct Billing;

    #[chromatofoor::actor(scope = Billing, type = human)]
    pub struct Clerk;

    #[chromatofoor::value_object(scope = Billing)]
    #[derive(Default)]
    pub struct LineItem {
        pub label: String,
        pub amount_cents: u64,
    }

    #[chromatofoor::aggregate(scope = Billing, identified_by = state(invoice_id))]
    #[derive(Clone, Default)]
    pub struct Invoice {
        pub invoice_id: Option<chromatofoor::Identifier>,
        #[allow(dead_code)]
        pub item: LineItem,
    }

    #[chromatofoor::event(scope = Invoice)]
    pub enum InvoiceEvent {
        InvoiceCreated {
            invoice_id: chromatofoor::Identifier,
            item: LineItem,
        },
    }

    impl Event<Invoice> for InvoiceCreated {
        fn evolve(&self, _state: Invoice) -> Invoice {
            Invoice {
                invoice_id: Some(self.invoice_id.clone()),
                item: self.item.clone(),
            }
        }
    }

    #[chromatofoor::command(scope = Invoice, creates, actors = [Clerk])]
    pub struct CreateInvoice {
        pub invoice_id: chromatofoor::Identifier,
        pub item: LineItem,
    }

    impl Command<Invoice> for CreateInvoice {
        type Error = &'static str;

        async fn decide(self, _state: &Invoice) -> Result<Vec<InvoiceEvent>, Self::Error> {
            Ok(vec![
                InvoiceCreated {
                    invoice_id: self.invoice_id,
                    item: self.item,
                }
                .into(),
            ])
        }
    }

    #[chromatofoor::read_model(scope = Billing, events = [InvoiceCreated])]
    pub struct InvoiceLedger {
        pub entries: InMemoryView<String, (String, u64)>,
    }

    impl Project<InvoiceCreated> for InvoiceLedger {
        async fn project(&self, event: Envelope<InvoiceCreated>) -> Result<(), ViewError> {
            self.entries.insert(
                &event,
                (
                    event.data.invoice_id.to_string(),
                    event.data.item.amount_cents,
                ),
            )?;
            Ok(())
        }
    }

    #[chromatofoor::query(scope = Billing, actors = [Clerk])]
    pub struct GetInvoiceAmount {
        pub invoice_id: String,
        pub bonus_cents: Option<u64>,
    }

    impl Query<InvoiceLedger> for GetInvoiceAmount {
        type Output = Option<u64>;
        type Error = std::convert::Infallible;

        async fn execute(self, read_model: &InvoiceLedger) -> Result<Self::Output, Self::Error> {
            let entry = read_model.entries.get(&self.invoice_id).await?;
            Ok(entry.map(|(_, amt)| amt + self.bonus_cents.unwrap_or(0)))
        }
    }

    struct CustomHealthProtocol;

    impl HttpProtocol for CustomHealthProtocol {
        fn name(&self) -> &'static str {
            "health"
        }

        fn router(self, runtime: Arc<DomainRuntime>) -> Router {
            let domain = runtime.domain_name();
            Router::new().route(
                "/custom/health",
                get(move || async move { Json(json!({ "status": "ok", "domain": domain })) }),
            )
        }
    }

    #[tokio::test]
    async fn http_server_merges_attached_protocols_and_serves_openapi_catalog_views_and_sse() {
        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://demo.local").build());
        let ledger = Arc::new(InvoiceLedger {
            entries: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
        });
        let runtime = Arc::new(
            DomainBuilder::<DemoDomain>::new(store)
                .register_command::<CreateInvoice, _>()
                .register_read_model(Arc::clone(&ledger))
                .register_query::<GetInvoiceAmount, _>(Arc::clone(&ledger))
                .build()
                .await
                .unwrap(),
        );

        let server = HttpServer::bind(3000).attach(CustomHealthProtocol);
        assert_eq!(server.attached_protocols(), vec!["health"]);

        let app = server.into_router(Arc::clone(&runtime));

        // 1. Custom attached HttpProtocol route works
        let req = Request::builder()
            .method("GET")
            .uri("/custom/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body_json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body_json,
            json!({ "status": "ok", "domain": "demo-domain" })
        );

        // 2. Built-in /v0/catalog route works
        let req = Request::builder()
            .method("GET")
            .uri("/v0/catalog")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let catalog_json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(catalog_json["domain"], "demo-domain");

        // 3. /openapi.json recursively expands $defs (`LineItem` object)
        let req = Request::builder()
            .method("GET")
            .uri("/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let openapi_json: Value = serde_json::from_slice(&bytes).unwrap();
        let item_schema = &openapi_json["paths"]["/v0/billing/invoice/create-invoice"]["post"]["requestBody"]
            ["content"]["application/json"]["schema"]["properties"]["item"];
        assert_eq!(item_schema["type"], "object");
        assert_eq!(item_schema["properties"]["amount_cents"]["type"], "integer");

        // 4. Dispatch command with `x-metadata-actor-type` header and query via `/v0/{bc}/views/{rm}/{query}/{id}` with nullable integer param
        let post_req = Request::builder()
            .method("POST")
            .uri("/v0/billing/invoice/create-invoice/inv-100")
            .header("content-type", "application/json")
            .header("x-metadata-actor-type", "clerk")
            .body(Body::from(
                json!({ "item": { "label": "Consulting", "amount_cents": 5000 } }).to_string(),
            ))
            .unwrap();
        let post_resp = app.clone().oneshot(post_req).await.unwrap();
        assert_eq!(post_resp.status(), StatusCode::OK);

        runtime.wait_caught_up().await.unwrap();
        let query_req = Request::builder()
            .method("GET")
            .uri("/v0/billing/views/invoice-ledger/get-invoice-amount/inv-100?bonus_cents=250")
            .header("x-metadata-actor-type", "clerk")
            .body(Body::empty())
            .unwrap();
        let query_resp = app.clone().oneshot(query_req).await.unwrap();
        assert_eq!(query_resp.status(), StatusCode::OK);
        let query_bytes = query_resp.into_body().collect().await.unwrap().to_bytes();
        let amount: Value = serde_json::from_slice(&query_bytes).unwrap();
        assert_eq!(amount, json!(5250));

        // 5. Stream SSE events from `GET /v0/events` with `subject_prefix` and `event_type` filters
        let sse_req = Request::builder()
            .method("GET")
            .uri(
                "/v0/events?subject_prefix=/demo-domain/billing/invoice&event_type=invoice-created",
            )
            .body(Body::empty())
            .unwrap();
        let sse_resp = app.oneshot(sse_req).await.unwrap();
        assert_eq!(sse_resp.status(), StatusCode::OK);
        let mut body = sse_resp.into_body();
        let frame = tokio::time::timeout(Duration::from_secs(2), body.frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let data_bytes = frame.into_data().unwrap();
        let sse_text = String::from_utf8_lossy(&data_bytes);
        assert!(sse_text.contains("invoice-created"), "{sse_text}");
        assert!(sse_text.contains("inv-100"), "{sse_text}");
    }
}
