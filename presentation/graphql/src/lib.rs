//! Dynamic GraphQL schema builder and Axum router for [`chromatofoor::domain::DomainRuntime`].
//!
//! Derives GraphQL `Query` and `Mutation` fields directly from the registered
//! [`CommandDescriptor`](chromatofoor::runtime::CommandDescriptor)s and
//! [`QueryDescriptor`](chromatofoor::runtime::QueryDescriptor)s on a [`DomainRuntime`].
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use chromatofoor::{
//!     architecturekit::{
//!         self, Envelope, InMemoryEventStore, InMemoryView, KeyedView, Store, ViewError,
//!     },
//!     bounded_context::{Command, Event, Project, Query},
//!     domain::DomainBuilder,
//! };
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
//! let schema = chromatofoor_graphql::build_schema(Arc::clone(&runtime)).unwrap();
//!
//! // 1. Execute GraphQL mutation:
//! let res = schema
//!     .execute(
//!         r#"mutation { orderingOrderPlaceOrder(payload: { orderId: "ord-1", totalCents: 4200 }, actor: "customer") }"#,
//!     )
//!     .await;
//! assert!(res.errors.is_empty(), "{:?}", res.errors);
//!
//! // 2. Execute GraphQL query:
//! runtime.wait_caught_up().await.unwrap();
//! let res = schema
//!     .execute(r#"{ orderingOrderTotalById(params: { orderId: "ord-1" }, actor: "customer") }"#)
//!     .await;
//! assert!(res.errors.is_empty(), "{:?}", res.errors);
//! assert_eq!(
//!     res.data.into_json().unwrap(),
//!     serde_json::json!({ "orderingOrderTotalById": 4200 })
//! );
//! # });
//! ```

pub mod type_mapper;

use std::{collections::HashMap, sync::Arc};

use async_graphql::dynamic::{
    Field, FieldFuture, FieldValue, InputValue, Object, Scalar, Schema, SchemaError, Subscription,
    SubscriptionField, SubscriptionFieldFuture, TypeRef,
};
use chromatofoor::{architecturekit::eventsourcingdb::Event as StoredEvent, domain::DomainRuntime};
use convert_case::{Case, Casing};
use serde_json::{Value, json};

fn collect_key_mappings(schema: &Value, mappings: &mut HashMap<String, String>) {
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        for (prop_name, prop_val) in props {
            mappings.insert(prop_name.to_case(Case::Camel), prop_name.clone());
            collect_key_mappings(prop_val, mappings);
        }
    }
    if let Some(items) = schema.get("items") {
        collect_key_mappings(items, mappings);
    }
    if let Some(defs) = schema
        .get("$defs")
        .or_else(|| schema.get("definitions"))
        .and_then(Value::as_object)
    {
        for def_val in defs.values() {
            collect_key_mappings(def_val, mappings);
        }
    }
    for combiner in ["allOf", "anyOf", "oneOf"] {
        if let Some(arr) = schema.get(combiner).and_then(Value::as_array) {
            for branch in arr {
                collect_key_mappings(branch, mappings);
            }
        }
    }
}

fn build_domain_event_object() -> Object {
    Object::new("DomainEvent")
        .field(Field::new(
            "id",
            TypeRef::named_nn(TypeRef::STRING),
            |ctx| {
                FieldFuture::new(async move {
                    let ev = ctx.parent_value.try_downcast_ref::<StoredEvent>()?;
                    Ok(Some(async_graphql::Value::from(ev.id().to_owned())))
                })
            },
        ))
        .field(Field::new(
            "subject",
            TypeRef::named_nn(TypeRef::STRING),
            |ctx| {
                FieldFuture::new(async move {
                    let ev = ctx.parent_value.try_downcast_ref::<StoredEvent>()?;
                    Ok(Some(async_graphql::Value::from(ev.subject().to_owned())))
                })
            },
        ))
        .field(Field::new(
            "ty",
            TypeRef::named_nn(TypeRef::STRING),
            |ctx| {
                FieldFuture::new(async move {
                    let ev = ctx.parent_value.try_downcast_ref::<StoredEvent>()?;
                    Ok(Some(async_graphql::Value::from(ev.ty().to_owned())))
                })
            },
        ))
        .field(Field::new("data", TypeRef::named_nn("JSON"), |ctx| {
            FieldFuture::new(async move {
                let ev = ctx.parent_value.try_downcast_ref::<StoredEvent>()?;
                Ok(Some(FieldValue::value(async_graphql::Value::from_json(
                    ev.data().clone(),
                )?)))
            })
        }))
}

/// Executes a GraphQL query or mutation string against `schema` with an optional actor identity
/// injected into the request context.
pub async fn execute_as(
    schema: &Schema,
    query: &str,
    actor: Option<&str>,
) -> async_graphql::Response {
    let mut req = async_graphql::Request::new(query);
    if let Some(a) = actor {
        let mut meta = HashMap::new();
        meta.insert("actor_type".to_owned(), a.to_owned());
        req = req.data(meta);
    }
    schema.execute(req).await
}

/// Builds an [`async_graphql::dynamic::Schema`] from a [`DomainRuntime`].
pub fn build_schema(runtime: Arc<DomainRuntime>) -> Result<Schema, SchemaError> {
    let mut schema_builder = Schema::build("Query", Some("Mutation"), Some("Subscription"));

    let mut query = Object::new("Query").field(Field::new(
        "ping",
        TypeRef::named_nn(TypeRef::STRING),
        |_| FieldFuture::new(async move { Ok(Some(async_graphql::Value::from("pong"))) }),
    ));
    let mut mutation = Object::new("Mutation");

    let json_scalar = Scalar::new("JSON");
    schema_builder = schema_builder
        .register(json_scalar)
        .register(build_domain_event_object());

    let rt_sub = Arc::clone(&runtime);
    let subscription = Subscription::new("Subscription").field(
        SubscriptionField::new("events", TypeRef::named_nn("DomainEvent"), move |ctx| {
            let rt = Arc::clone(&rt_sub);
            let subject_filter = ctx
                .args
                .get("subjectPrefix")
                .and_then(|v| v.string().ok())
                .map(ToOwned::to_owned);
            let ty_filter = ctx
                .args
                .get("eventType")
                .and_then(|v| v.string().ok())
                .map(ToOwned::to_owned);

            SubscriptionFieldFuture::new(async move {
                let mut rx = rt.subscribe_events();
                let stream = async_stream::stream! {
                    while let Some(ev) = rx.recv().await {
                        if let Some(ref prefix) = subject_filter
                            && !ev.subject().starts_with(prefix)
                        {
                            continue;
                        }
                        if let Some(ref ty) = ty_filter
                            && !ev.ty().contains(ty)
                        {
                            continue;
                        }
                        yield Ok(FieldValue::owned_any(ev));
                    }
                };
                Ok(stream)
            })
        })
        .description("Streams live domain events published to the DomainRuntime.")
        .argument(InputValue::new(
            "subjectPrefix",
            TypeRef::named(TypeRef::STRING),
        ))
        .argument(InputValue::new(
            "eventType",
            TypeRef::named(TypeRef::STRING),
        )),
    );

    let mut key_mappings = HashMap::new();
    for cmd in runtime.commands() {
        collect_key_mappings(&cmd.schema, &mut key_mappings);
    }
    for q in runtime.queries() {
        collect_key_mappings(&q.parameters_schema, &mut key_mappings);
        collect_key_mappings(&q.output_schema, &mut key_mappings);
    }
    let key_mappings = Arc::new(key_mappings);

    for cmd in runtime.commands() {
        let has_props = cmd
            .schema
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|p| !p.is_empty());

        let input_name = format!(
            "{}{}{}Input",
            cmd.bounded_context.to_case(Case::Pascal),
            cmd.target.to_case(Case::Pascal),
            cmd.name.to_case(Case::Pascal)
        );
        if has_props {
            for input_obj in type_mapper::create_input_objects(&input_name, &cmd.schema) {
                schema_builder = schema_builder.register(input_obj);
            }
        }

        let field_name =
            format!("{}_{}_{}", cmd.bounded_context, cmd.target, cmd.name).to_case(Case::Camel);
        let rt = Arc::clone(&runtime);
        let cmd_desc = cmd.clone();
        let mappings = Arc::clone(&key_mappings);

        let mut field = Field::new(field_name, TypeRef::named_nn(TypeRef::STRING), move |ctx| {
            let rt = Arc::clone(&rt);
            let cmd_desc = cmd_desc.clone();
            let mappings = Arc::clone(&mappings);
            FieldFuture::new(async move {
                let mut payload_val = if let Some(val) = ctx.args.get("payload") {
                    val.deserialize::<Value>().unwrap_or_else(|_| json!({}))
                } else {
                    json!({})
                };
                map_json_keys(&mut payload_val, &mappings);

                let arg_actor = ctx
                    .args
                    .get("actor")
                    .and_then(|v| v.string().ok())
                    .map(ToOwned::to_owned);
                let ctx_actor = ctx
                    .data_opt::<HashMap<String, String>>()
                    .and_then(|m| m.get("actor_type").or_else(|| m.get("actor")))
                    .cloned();
                let actor = arg_actor.or(ctx_actor);

                rt.dispatch_command(
                    cmd_desc.bounded_context,
                    cmd_desc.target,
                    cmd_desc.name,
                    payload_val,
                    actor.as_deref(),
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                Ok(Some(async_graphql::Value::from("Success")))
            })
        })
        .description(cmd.description);

        if has_props {
            field = field.argument(InputValue::new("payload", TypeRef::named_nn(&input_name)));
        }
        field = field.argument(InputValue::new("actor", TypeRef::named(TypeRef::STRING)));

        mutation = mutation.field(field);
    }

    for q in runtime.queries() {
        let has_params = q
            .parameters_schema
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|p| !p.is_empty());

        let params_input_name = format!(
            "{}{}ParamsInput",
            q.bounded_context.to_case(Case::Pascal),
            q.name.to_case(Case::Pascal)
        );
        if has_params {
            for input_obj in
                type_mapper::create_input_objects(&params_input_name, &q.parameters_schema)
            {
                schema_builder = schema_builder.register(input_obj);
            }
        }

        let output_type_name = format!(
            "{}{}Output",
            q.bounded_context.to_case(Case::Pascal),
            q.name.to_case(Case::Pascal)
        );
        let (output_type_ref, output_shape, output_objects) =
            type_mapper::build_query_output_type(&output_type_name, &q.output_schema);
        for out_obj in output_objects {
            schema_builder = schema_builder.register(out_obj);
        }

        let field_name = format!("{}_{}", q.bounded_context, q.name).to_case(Case::Camel);
        let rt = Arc::clone(&runtime);
        let q_desc = q.clone();
        let mappings = Arc::clone(&key_mappings);

        let mut field = Field::new(field_name, output_type_ref, move |ctx| {
            let rt = Arc::clone(&rt);
            let q_desc = q_desc.clone();
            let mappings = Arc::clone(&mappings);
            FieldFuture::new(async move {
                let mut params_val = if let Some(val) = ctx.args.get("params") {
                    val.deserialize::<Value>().unwrap_or_else(|_| json!({}))
                } else {
                    json!({})
                };
                map_json_keys(&mut params_val, &mappings);

                let arg_actor = ctx
                    .args
                    .get("actor")
                    .and_then(|v| v.string().ok())
                    .map(ToOwned::to_owned);
                let ctx_actor = ctx
                    .data_opt::<HashMap<String, String>>()
                    .and_then(|m| m.get("actor_type").or_else(|| m.get("actor")))
                    .cloned();
                let actor = arg_actor.or(ctx_actor);

                let result = rt
                    .dispatch_query(
                        q_desc.bounded_context,
                        q_desc.name,
                        params_val,
                        actor.as_deref(),
                    )
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                type_mapper::json_to_field_value(result, output_shape)
            })
        })
        .description(q.description);

        if has_params {
            field = field.argument(InputValue::new(
                "params",
                TypeRef::named_nn(&params_input_name),
            ));
        }
        field = field.argument(InputValue::new("actor", TypeRef::named(TypeRef::STRING)));

        query = query.field(field);
    }

    schema_builder = schema_builder
        .register(query)
        .register(mutation)
        .register(subscription);
    schema_builder.finish()
}

/// GraphQL HTTP protocol adapter implementing [`chromatofoor_http::HttpProtocol`].
///
/// Mounts:
/// - `GET /` and `GET /graphiql` — Interactive GraphiQL IDE.
/// - `POST /graphql` and `GET /graphql?query=...` — GraphQL queries, mutations, and SSE streams.
/// - `GET /graphql/sse` and `POST /graphql/sse` — GraphQL over Server-Sent Events (SSE) subscriptions.
/// - `/graphql/ws` — GraphQL over WebSocket subscriptions.
#[cfg(feature = "http")]
#[derive(Clone, Copy, Debug, Default)]
pub struct GraphQL;

#[cfg(feature = "http")]
impl GraphQL {
    /// Creates a new [`GraphQL`] HTTP protocol adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "http")]
impl chromatofoor_http::HttpProtocol for GraphQL {
    fn name(&self) -> &'static str {
        "graphql"
    }

    fn router(self, runtime: Arc<DomainRuntime>) -> axum::Router {
        routes(runtime)
    }
}

#[cfg(feature = "http")]
fn attach_header_metadata(
    mut request: async_graphql::Request,
    headers: &axum::http::HeaderMap,
) -> async_graphql::Request {
    let mut metadata = HashMap::<String, String>::new();

    if let Some(actor) = chromatofoor_http::actor_from_headers(headers) {
        metadata.insert("actor_type".to_owned(), actor.to_owned());
    }
    for (name, value) in headers {
        if let Some(meta_key) = name.as_str().strip_prefix("x-metadata-")
            && let Ok(val) = value.to_str()
        {
            metadata.insert(meta_key.replace('-', "_"), val.to_owned());
        }
    }
    request = request.data(metadata);
    request
}

#[cfg(feature = "http")]
fn execute_sse_stream(schema: Schema, request: async_graphql::Request) -> axum::response::Response {
    use std::convert::Infallible;

    use axum::response::{
        IntoResponse,
        sse::{Event, Sse},
    };
    use tokio_stream::StreamExt as _;

    let stream = schema
        .execute_stream(request)
        .map(|resp| {
            let payload = serde_json::to_string(&resp).unwrap_or_default();
            Ok::<_, Infallible>(Event::default().event("next").data(payload))
        })
        .chain(tokio_stream::once(Ok::<_, Infallible>(
            Event::default().event("complete").data(""),
        )));
    Sse::new(stream).into_response()
}

#[cfg(feature = "http")]
fn request_from_query_params(params: &HashMap<String, String>) -> Option<async_graphql::Request> {
    let query_str = params.get("query")?;
    let mut req = async_graphql::Request::new(query_str.clone());
    if let Some(op_name) = params
        .get("operationName")
        .or_else(|| params.get("operation_name"))
    {
        req = req.operation_name(op_name.clone());
    }
    if let Some(vars_raw) = params.get("variables")
        && let Ok(vars_json) = serde_json::from_str::<Value>(vars_raw)
    {
        req = req.variables(async_graphql::Variables::from_json(vars_json));
    }
    Some(req)
}

#[cfg(feature = "http")]
fn prefers_event_stream(headers: &axum::http::HeaderMap, query: &str) -> bool {
    let header_sse = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| {
            accept.contains("text/event-stream") && !accept.contains("application/json")
        });
    header_sse || query.trim_start().starts_with("subscription")
}

/// Builds an [`axum::Router`] serving GraphiQL at `/` and `/graphiql`, GraphQL queries/mutations
/// at `GET /graphql` and `POST /graphql`, SSE subscriptions at `/graphql/sse`, and WebSocket
/// subscriptions at `/graphql/ws`.
#[cfg(feature = "http")]
#[allow(clippy::expect_used)]
pub fn routes(runtime: Arc<DomainRuntime>) -> axum::Router {
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
    use axum::{
        Extension, Router, extract::Query as AxumQuery, response::IntoResponse, routing::get,
    };

    async fn graphql_post_handler(
        Extension(schema): Extension<Schema>,
        headers: axum::http::HeaderMap,
        req: GraphQLRequest,
    ) -> axum::response::Response {
        let request = attach_header_metadata(req.into_inner(), &headers);
        if prefers_event_stream(&headers, &request.query) {
            return execute_sse_stream(schema, request);
        }
        let resp: GraphQLResponse = schema.execute(request).await.into();
        resp.into_response()
    }

    async fn graphql_get_handler(
        Extension(schema): Extension<Schema>,
        headers: axum::http::HeaderMap,
        AxumQuery(params): AxumQuery<HashMap<String, String>>,
    ) -> axum::response::Response {
        let Some(raw_req) = request_from_query_params(&params) else {
            return graphiql().await.into_response();
        };
        let request = attach_header_metadata(raw_req, &headers);
        if prefers_event_stream(&headers, &request.query) {
            return execute_sse_stream(schema, request);
        }
        let resp: GraphQLResponse = schema.execute(request).await.into();
        resp.into_response()
    }

    async fn graphql_sse_get_handler(
        Extension(schema): Extension<Schema>,
        headers: axum::http::HeaderMap,
        AxumQuery(params): AxumQuery<HashMap<String, String>>,
    ) -> axum::response::Response {
        let Some(raw_req) = request_from_query_params(&params) else {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Missing `query` query parameter",
            )
                .into_response();
        };
        let request = attach_header_metadata(raw_req, &headers);
        execute_sse_stream(schema, request)
    }

    async fn graphql_sse_post_handler(
        Extension(schema): Extension<Schema>,
        headers: axum::http::HeaderMap,
        req: GraphQLRequest,
    ) -> axum::response::Response {
        let request = attach_header_metadata(req.into_inner(), &headers);
        execute_sse_stream(schema, request)
    }

    async fn graphiql() -> axum::response::Html<String> {
        axum::response::Html(
            async_graphql::http::GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql/ws")
                .finish(),
        )
    }

    let schema = build_schema(runtime).expect("Failed to build GraphQL Schema");

    Router::new()
        .route("/", get(graphiql))
        .route("/graphiql", get(graphiql))
        .route(
            "/graphql",
            get(graphql_get_handler).post(graphql_post_handler),
        )
        .route(
            "/graphql/sse",
            get(graphql_sse_get_handler).post(graphql_sse_post_handler),
        )
        .route_service("/graphql/ws", GraphQLSubscription::new(schema.clone()))
        .layer(Extension(schema))
}

fn map_json_keys(val: &mut Value, mappings: &HashMap<String, String>) {
    match val {
        Value::Object(map) => {
            let mut updated_map = serde_json::Map::new();
            let old_map = std::mem::take(map);
            for (k, mut v) in old_map {
                map_json_keys(&mut v, mappings);
                let new_key = mappings.get(&k).cloned().unwrap_or(k);
                updated_map.insert(new_key, v);
            }
            *map = updated_map;
        }
        Value::Array(arr) => {
            for v in arr {
                map_json_keys(v, mappings);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, dead_code)]
mod tests {
    use std::sync::Arc;

    use chromatofoor::{
        architecturekit::{
            Envelope, InMemoryEventStore, InMemoryView, KeyedView, Store, ViewError,
        },
        bounded_context::{Command, Event, Project, Query},
        domain::DomainBuilder,
    };
    use serde_json::json;

    use super::*;

    #[chromatofoor::domain]
    pub struct ShopDomain;

    #[chromatofoor::bounded_context(scope = ShopDomain)]
    pub struct Catalog;

    #[chromatofoor::actor(scope = Catalog, type = human)]
    pub struct Merchant;

    #[chromatofoor::value_object(scope = Catalog)]
    #[derive(Default)]
    pub struct Dimensions {
        pub width_cm: u32,
        pub height_cm: u32,
    }

    #[chromatofoor::aggregate(scope = Catalog, identified_by = state(sku))]
    #[derive(Clone, Default)]
    pub struct Product {
        pub sku: Option<chromatofoor::Identifier>,
        pub dimensions: Dimensions,
    }

    #[chromatofoor::event(scope = Product)]
    pub enum ProductEvent {
        ProductListed {
            sku: chromatofoor::Identifier,
            dimensions: Dimensions,
        },
    }

    impl Event<Product> for ProductListed {
        fn evolve(&self, _state: Product) -> Product {
            Product {
                sku: Some(self.sku.clone()),
                dimensions: self.dimensions.clone(),
            }
        }
    }

    #[chromatofoor::command(scope = Product)]
    pub enum ProductCommand {
        #[lifecycle(create)]
        #[actors(Merchant)]
        ListProduct {
            sku: chromatofoor::Identifier,
            dimensions: Dimensions,
        },
    }

    impl Command<Product> for ListProduct {
        type Error = &'static str;

        async fn decide(self, _state: &Product) -> Result<Vec<ProductEvent>, Self::Error> {
            Ok(vec![
                ProductListed {
                    sku: self.sku,
                    dimensions: self.dimensions,
                }
                .into(),
            ])
        }
    }

    #[chromatofoor::value_object(scope = Catalog)]
    pub struct ProductSummary {
        pub sku: String,
        pub dimensions: Dimensions,
        pub attributes: std::collections::HashMap<String, serde_json::Value>,
    }

    #[chromatofoor::read_model(scope = Catalog, events = [ProductListed])]
    pub struct ProductCatalog {
        pub products: InMemoryView<String, ProductSummary>,
    }

    impl Project<ProductListed> for ProductCatalog {
        async fn project(&self, event: Envelope<ProductListed>) -> Result<(), ViewError> {
            let mut attributes = std::collections::HashMap::new();
            attributes.insert("featured".to_owned(), json!(true));
            self.products.insert(
                &event,
                ProductSummary {
                    sku: event.data.sku.to_string(),
                    dimensions: event.data.dimensions.clone(),
                    attributes,
                },
            )?;
            Ok(())
        }
    }

    #[chromatofoor::query(scope = Catalog, actors = [Merchant])]
    pub struct GetProductBySku {
        pub sku: String,
    }

    impl Query<ProductCatalog> for GetProductBySku {
        type Output = Option<ProductSummary>;
        type Error = std::convert::Infallible;

        async fn execute(self, read_model: &ProductCatalog) -> Result<Self::Output, Self::Error> {
            Ok(read_model.products.get(&self.sku).await?)
        }
    }

    #[chromatofoor::query(scope = Catalog, actors = [Merchant])]
    pub struct GetProductsBatch {
        pub skus: Vec<String>,
    }

    impl Query<ProductCatalog> for GetProductsBatch {
        type Output = Vec<Option<ProductSummary>>;
        type Error = std::convert::Infallible;

        async fn execute(self, read_model: &ProductCatalog) -> Result<Self::Output, Self::Error> {
            let mut results = Vec::with_capacity(self.skus.len());
            for sku in &self.skus {
                results.push(read_model.products.get(sku).await?);
            }
            Ok(results)
        }
    }

    #[tokio::test]
    async fn nested_input_and_output_objects_and_actor_enforcement_work() {
        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://shop.local").build());
        let catalog_view = Arc::new(ProductCatalog {
            products: InMemoryView::new(|p: &ProductSummary| p.sku.clone()),
        });
        let runtime = Arc::new(
            DomainBuilder::<ShopDomain>::new(store)
                .register_command::<ListProduct, _>()
                .register_read_model(Arc::clone(&catalog_view))
                .register_query::<GetProductBySku, _>(Arc::clone(&catalog_view))
                .register_query::<GetProductsBatch, _>(Arc::clone(&catalog_view))
                .build()
                .await
                .unwrap(),
        );
        let schema = build_schema(Arc::clone(&runtime)).unwrap();

        // 1. Unauthorized actor is rejected
        let unauthorized = execute_as(
            &schema,
            r#"mutation {
                catalogProductListProduct(
                    payload: { sku: "SKU-1", dimensions: { widthCm: 10, heightCm: 20 } }
                )
            }"#,
            Some("guest"),
        )
        .await;
        assert!(!unauthorized.errors.is_empty());

        // 2. Authorized actor with nested InputObject succeeds
        let authorized = execute_as(
            &schema,
            r#"mutation {
                catalogProductListProduct(
                    payload: { sku: "SKU-1", dimensions: { widthCm: 10, heightCm: 20 } }
                )
            }"#,
            Some("merchant"),
        )
        .await;
        assert!(authorized.errors.is_empty(), "{:?}", authorized.errors);
        assert_eq!(
            authorized.data.into_json().unwrap(),
            json!({ "catalogProductListProduct": "Success" })
        );

        // 3. Query returns typed GraphQL Output Object with nested Dimensions object and JSON scalar fallback for HashMap<String, Value>
        runtime.wait_caught_up().await.unwrap();
        let query_res = execute_as(
            &schema,
            r#"{
                catalogGetProductBySku(params: { sku: "SKU-1" }) {
                    sku
                    attributes
                    dimensions {
                        widthCm
                        heightCm
                    }
                }
            }"#,
            Some("merchant"),
        )
        .await;
        assert!(query_res.errors.is_empty(), "{:?}", query_res.errors);
        assert_eq!(
            query_res.data.into_json().unwrap(),
            json!({
                "catalogGetProductBySku": {
                    "sku": "SKU-1",
                    "attributes": { "featured": true },
                    "dimensions": {
                        "widthCm": 10,
                        "heightCm": 20
                    }
                }
            })
        );

        // 4. Query returning `Vec<Option<ProductSummary>>` handles `[Some(product), None]` without failing on null item
        let batch_res = execute_as(
            &schema,
            r#"{
                catalogGetProductsBatch(params: { skus: ["SKU-1", "SKU-MISSING"] }) {
                    sku
                    dimensions {
                        widthCm
                    }
                }
            }"#,
            Some("merchant"),
        )
        .await;
        assert!(batch_res.errors.is_empty(), "{:?}", batch_res.errors);
        assert_eq!(
            batch_res.data.into_json().unwrap(),
            json!({
                "catalogGetProductsBatch": [
                    {
                        "sku": "SKU-1",
                        "dimensions": { "widthCm": 10 }
                    }
                ]
            })
        );
    }

    #[test]
    fn cyclic_json_schema_falls_back_to_json_scalar_without_stack_overflow() {
        let cyclic_schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "child": { "$ref": "#/$defs/Node" }
            },
            "required": ["name"],
            "$defs": {
                "Node": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "child": { "$ref": "#/$defs/Node" }
                    },
                    "required": ["name"]
                }
            }
        });
        let (_type_ref, shape, objects) =
            type_mapper::build_query_output_type("CyclicTree", &cyclic_schema);
        assert!(matches!(shape, type_mapper::OutputShape::Object));
        assert!(!objects.is_empty());
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn graphql_http_protocol_supports_get_and_sse_on_http_server() {
        use std::time::Duration;

        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use chromatofoor_http::HttpServer;
        use http_body_util::BodyExt as _;
        use tower::ServiceExt as _;

        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://shop.local").build());
        let runtime = Arc::new(
            DomainBuilder::<ShopDomain>::new(store)
                .register_command::<ListProduct, _>()
                .build()
                .await
                .unwrap(),
        );

        let app = HttpServer::bind(3000)
            .attach(GraphQL::new())
            .into_router(Arc::clone(&runtime));

        // 1. GET /graphql?query={ping} executes GraphQL query over HTTP GET
        let get_req = Request::builder()
            .method("GET")
            .uri("/graphql?query=%7Bping%7D")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(get_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body_json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body_json["data"]["ping"], "pong");

        // 2. GET /graphql/sse?query={ping} streams GraphQL response over SSE
        let sse_req = Request::builder()
            .method("GET")
            .uri("/graphql/sse?query=%7Bping%7D")
            .body(Body::empty())
            .unwrap();
        let sse_resp = app.clone().oneshot(sse_req).await.unwrap();
        assert_eq!(sse_resp.status(), StatusCode::OK);
        assert!(
            sse_resp
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("text/event-stream")
        );
        let sse_bytes = sse_resp.into_body().collect().await.unwrap().to_bytes();
        let sse_text = String::from_utf8_lossy(&sse_bytes);
        assert!(sse_text.contains("event: next"), "{sse_text}");
        assert!(sse_text.contains("\"ping\":\"pong\""), "{sse_text}");
        assert!(sse_text.contains("event: complete"), "{sse_text}");

        // 3. POST /graphql/sse streams live `subscription { events(...) }` frames when a mutation is executed
        let sub_req = Request::builder()
            .method("POST")
            .uri("/graphql/sse")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "query": "subscription { events(subjectPrefix: \"/shop-domain/catalog/product\") { id subject ty data } }"
                })
                .to_string(),
            ))
            .unwrap();
        let sub_resp = app.clone().oneshot(sub_req).await.unwrap();
        assert_eq!(sub_resp.status(), StatusCode::OK);
        let mut sub_body = sub_resp.into_body();

        let mut_req = Request::builder()
            .method("POST")
            .uri("/graphql")
            .header("content-type", "application/json")
            .header("x-actor-type", "merchant")
            .body(Body::from(
                json!({
                    "query": "mutation { catalogProductListProduct(payload: { sku: \"SKU-LIVE\", dimensions: { widthCm: 5, heightCm: 5 } }) }"
                })
                .to_string(),
            ))
            .unwrap();
        let mut_resp = app.oneshot(mut_req).await.unwrap();
        assert_eq!(mut_resp.status(), StatusCode::OK);

        let frame = tokio::time::timeout(Duration::from_secs(2), sub_body.frame())
            .await
            .expect("timed out waiting for GraphQL SSE subscription frame")
            .expect("GraphQL SSE subscription stream ended")
            .expect("GraphQL SSE subscription frame error");
        let frame_text = String::from_utf8_lossy(frame.data_ref().unwrap());
        assert!(frame_text.contains("event: next"), "{frame_text}");
        assert!(frame_text.contains("SKU-LIVE"), "{frame_text}");
    }
}
