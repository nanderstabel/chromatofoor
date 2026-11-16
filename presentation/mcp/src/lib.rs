//! Model Context Protocol (MCP) JSON-RPC presentation adapter for
//! [`chromatofoor::domain::DomainRuntime`].
//!
//! Exposes all registered commands and queries as MCP tools (`tools/list`, `tools/call`) with
//! JSON Schemas derived automatically via [`chromatofoor::architecturekit::Schema`].
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
//! use serde_json::json;
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
//! /// Looks up the total for a single order.
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
//! // 1. Inspect generated MCP tools:
//! let tools_resp = chromatofoor_mcp::handle_request(
//!     &runtime,
//!     &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }),
//! )
//! .await
//! .unwrap();
//! assert_eq!(tools_resp["result"]["tools"].as_array().unwrap().len(), 2);
//!
//! // 2. Invoke command via `tools/call`:
//! let call_resp = chromatofoor_mcp::handle_request(
//!     &runtime,
//!     &json!({
//!         "jsonrpc": "2.0",
//!         "id": 2,
//!         "method": "tools/call",
//!         "params": {
//!             "name": "invoke_ordering_order_place-order",
//!             "arguments": { "order_id": "ord-1", "total_cents": 4200, "_actor": "customer" }
//!         }
//!     }),
//! )
//! .await
//! .unwrap();
//! assert!(call_resp.get("error").is_none(), "{call_resp}");
//!
//! // 3. Query read model via `tools/call`:
//! runtime.wait_caught_up().await.unwrap();
//! let query_resp = chromatofoor_mcp::handle_request(
//!     &runtime,
//!     &json!({
//!         "jsonrpc": "2.0",
//!         "id": 3,
//!         "method": "tools/call",
//!         "params": {
//!             "name": "query_ordering_order-total-by-id",
//!             "arguments": { "order_id": "ord-1", "_actor": "customer" }
//!         }
//!     }),
//! )
//! .await
//! .unwrap();
//! assert_eq!(query_resp["result"]["content"][0]["text"], "4200");
//! # });
//! ```

use std::sync::Arc;

use chromatofoor::{bounded_context::ActorType, domain::DomainRuntime};
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt as _,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
        ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult,
        Resource, ResourceContents, ResourceTemplate, ServerCapabilities, ServerConfig, Tool,
        ToolAnnotations,
    },
    service::RequestContext,
};
use serde_json::{Value, json};

fn enrich_schema_with_actor(schema: &Value, allowed_actors: &[(&str, ActorType)]) -> Value {
    let mut enriched = schema.clone();
    if let Some(obj) = enriched.as_object_mut() {
        let props = obj
            .entry("properties")
            .or_insert_with(|| json!({}))
            .as_object_mut();
        if let Some(props_map) = props {
            let actor_prop = if allowed_actors.is_empty() {
                json!({
                    "type": "string",
                    "description": "Optional actor role executing this operation."
                })
            } else {
                let actor_names: Vec<&str> = allowed_actors.iter().map(|(a, _)| *a).collect();
                json!({
                    "type": "string",
                    "enum": actor_names,
                    "description": "Actor role executing this operation."
                })
            };
            props_map.insert("_actor".to_owned(), actor_prop);
        }
    }
    enriched
}

fn strip_actor_field(mut val: Value) -> Value {
    if let Some(obj) = val.as_object_mut() {
        obj.remove("_actor");
    }
    val
}

/// Builds typed [`rmcp::model::Tool`] descriptors from `runtime`, including
/// [`ToolAnnotations`] (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`)
/// and `outputSchema` (for object-returning queries).
#[must_use]
pub fn build_mcp_tools(runtime: &DomainRuntime) -> Vec<Tool> {
    let mut tools = Vec::new();

    for cmd in runtime.commands() {
        let tool_name = format!("invoke_{}_{}_{}", cmd.bounded_context, cmd.target, cmd.name);
        let input_schema = Arc::new(rmcp::model::object(enrich_schema_with_actor(
            &cmd.schema,
            cmd.allowed_actors,
        )));
        let annotations = ToolAnnotations::with_title(format!(
            "{}/{}/{}",
            cmd.bounded_context, cmd.target, cmd.name
        ))
        .read_only(false)
        .destructive(false)
        .open_world(false);
        tools.push(
            Tool::new(tool_name, cmd.description, input_schema).with_annotations(annotations),
        );
    }

    for q in runtime.queries() {
        let tool_name = format!("query_{}_{}", q.bounded_context, q.name);
        let input_schema = Arc::new(rmcp::model::object(enrich_schema_with_actor(
            &q.parameters_schema,
            q.allowed_actors,
        )));
        let annotations = ToolAnnotations::with_title(format!("{}/{}", q.bounded_context, q.name))
            .read_only(true)
            .idempotent(true)
            .open_world(false);
        let mut tool =
            Tool::new(tool_name, q.description, input_schema).with_annotations(annotations);
        if q.output_schema.get("type").and_then(Value::as_str) == Some("object")
            && q.output_schema.is_object()
        {
            tool =
                tool.with_raw_output_schema(Arc::new(rmcp::model::object(q.output_schema.clone())));
        }
        tools.push(tool);
    }

    tools
}

/// Builds typed [`rmcp::model::Resource`] descriptors from `runtime`.
#[must_use]
pub fn build_mcp_resources(runtime: &DomainRuntime) -> Vec<Resource> {
    let mut resources = vec![
        Resource::new("esdm://catalog", "domain-catalog")
            .with_description("Full ESDM domain catalog of commands, queries, and actors.")
            .with_mime_type("application/json"),
    ];

    for q in runtime.queries() {
        resources.push(
            Resource::new(format!("esdm://{}/{}", q.bounded_context, q.name), q.name)
                .with_description(q.description)
                .with_mime_type("application/json"),
        );
    }

    resources
}

/// Builds typed [`rmcp::model::ResourceTemplate`] descriptors from `runtime`.
#[must_use]
pub fn build_mcp_resource_templates(runtime: &DomainRuntime) -> Vec<ResourceTemplate> {
    runtime
        .queries()
        .iter()
        .map(|q| {
            ResourceTemplate::new(
                format!("esdm://{}/{}/{{id}}", q.bounded_context, q.name),
                q.name,
            )
            .with_description(q.description)
            .with_mime_type("application/json")
        })
        .collect()
}

/// Builds the list of MCP tool definitions (`tools/list`) from `runtime` as JSON values.
#[must_use]
pub fn list_tools(runtime: &DomainRuntime) -> Vec<Value> {
    build_mcp_tools(runtime)
        .into_iter()
        .filter_map(|t| serde_json::to_value(t).ok())
        .collect()
}

/// Builds the list of MCP resources (`resources/list`) from `runtime` as JSON values.
#[must_use]
pub fn list_resources(runtime: &DomainRuntime) -> Vec<Value> {
    let mut resources = vec![json!({
        "uri": "esdm://catalog",
        "name": "domain-catalog",
        "description": "Full ESDM domain catalog of commands, queries, and actors.",
        "mimeType": "application/json",
    })];

    for q in runtime.queries() {
        resources.push(json!({
            "uri": format!("esdm://{}/{}", q.bounded_context, q.name),
            "uriTemplate": format!("esdm://{}/{}/{{id}}", q.bounded_context, q.name),
            "name": q.name,
            "description": q.description,
            "mimeType": "application/json",
        }));
    }

    resources
}

/// Builds the list of MCP resource templates (`resources/templates/list`) from `runtime` as JSON values.
#[must_use]
pub fn list_resource_templates(runtime: &DomainRuntime) -> Vec<Value> {
    build_mcp_resource_templates(runtime)
        .into_iter()
        .filter_map(|t| serde_json::to_value(t).ok())
        .collect()
}

async fn read_esdm_resource(
    runtime: &DomainRuntime,
    uri: &str,
    actor: Option<&str>,
) -> Result<String, String> {
    if uri == "esdm://catalog" {
        let catalog = runtime.catalog_json();
        return Ok(serde_json::to_string(&catalog).unwrap_or_default());
    }

    let mut parts = uri.strip_prefix("esdm://").unwrap_or("").split('/');
    let bounded_context = parts.next().unwrap_or("");
    let query_name = parts.next().unwrap_or("");
    let target_id = parts.next();

    let mut params = serde_json::Map::new();
    if let Some(id_val) = target_id
        && !id_val.is_empty()
        && id_val != "list"
    {
        if let Some(q_desc) = runtime
            .queries()
            .iter()
            .find(|q| q.bounded_context == bounded_context && q.name == query_name)
            && let Some(props) = q_desc
                .parameters_schema
                .get("properties")
                .and_then(Value::as_object)
        {
            let target_key = if props.contains_key("id") {
                Some("id".to_owned())
            } else {
                props
                    .keys()
                    .find(|k| k.ends_with("_id"))
                    .or_else(|| {
                        q_desc
                            .parameters_schema
                            .get("required")
                            .and_then(Value::as_array)
                            .and_then(|req| req.iter().find_map(Value::as_str))
                            .and_then(|k| props.get_key_value(k).map(|(key, _)| key))
                    })
                    .or_else(|| props.keys().next())
                    .cloned()
            };
            let key = target_key.unwrap_or_else(|| "id".to_owned());
            params.insert(key, Value::String(id_val.to_owned()));
        } else {
            params.insert("id".to_owned(), Value::String(id_val.to_owned()));
        }
    }

    runtime
        .dispatch_query(bounded_context, query_name, Value::Object(params), actor)
        .await
        .map(|val| serde_json::to_string(&val).unwrap_or_default())
        .map_err(|e| e.to_string())
}

enum ToolExecutionOutcome {
    CommandSuccess(usize),
    QuerySuccess(Value),
    DomainError(String),
    UnknownTool(String),
}

async fn execute_tool_call(
    runtime: &DomainRuntime,
    name: &str,
    args: Value,
) -> ToolExecutionOutcome {
    let actor = args
        .get("_actor")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    if let Some(cmd_desc) = runtime
        .commands()
        .iter()
        .find(|c| format!("invoke_{}_{}_{}", c.bounded_context, c.target, c.name) == name)
    {
        let payload = strip_actor_field(args.get("payload").cloned().unwrap_or(args));
        match runtime
            .dispatch_command(
                cmd_desc.bounded_context,
                cmd_desc.target,
                cmd_desc.name,
                payload,
                actor.as_deref(),
            )
            .await
        {
            Ok(events) => ToolExecutionOutcome::CommandSuccess(events.len()),
            Err(e) => ToolExecutionOutcome::DomainError(e.to_string()),
        }
    } else if let Some(q_desc) = runtime
        .queries()
        .iter()
        .find(|q| format!("query_{}_{}", q.bounded_context, q.name) == name)
    {
        let params = strip_actor_field(args.get("params").cloned().unwrap_or(args));
        match runtime
            .dispatch_query(
                q_desc.bounded_context,
                q_desc.name,
                params,
                actor.as_deref(),
            )
            .await
        {
            Ok(val) => ToolExecutionOutcome::QuerySuccess(val),
            Err(e) => ToolExecutionOutcome::DomainError(e.to_string()),
        }
    } else {
        ToolExecutionOutcome::UnknownTool(name.to_owned())
    }
}

/// Official `rmcp` MCP server handler wrapping a [`DomainRuntime`].
///
/// Implements [`rmcp::ServerHandler`] for the latest MCP specification (`2026-07-28`, with
/// backward compatibility for `2025-11-25` and `2024-11-05`), exposing all domain commands
/// and queries as typed MCP tools with [`ToolAnnotations`], `outputSchema`, `structuredContent`,
/// and `esdm://` resources/templates.
#[derive(Clone)]
pub struct McpDomainServer {
    runtime: Arc<DomainRuntime>,
}

impl McpDomainServer {
    /// Creates a new [`McpDomainServer`] wrapping `runtime`.
    #[must_use]
    pub fn new(runtime: Arc<DomainRuntime>) -> Self {
        Self { runtime }
    }

    /// Returns a reference to the underlying [`DomainRuntime`].
    #[must_use]
    pub fn runtime(&self) -> &Arc<DomainRuntime> {
        &self.runtime
    }
}

impl ServerHandler for McpDomainServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            self.runtime.domain_name(),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        build_mcp_tools(&self.runtime)
            .into_iter()
            .find(|t| t.name == name)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(build_mcp_tools(
            &self.runtime,
        )))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(build_mcp_resources(
            &self.runtime,
        )))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult::with_all_items(
            build_mcp_resource_templates(&self.runtime),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        match read_esdm_resource(&self.runtime, &request.uri, None).await {
            Ok(text) => Ok(ReadResourceResult::new(vec![
                ResourceContents::text(text, request.uri).with_mime_type("application/json"),
            ])
            .into()),
            Err(err) => Err(ErrorData::internal_error(err, None)),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        match execute_tool_call(&self.runtime, &request.name, args).await {
            ToolExecutionOutcome::CommandSuccess(event_count) => {
                Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                    "Command executed successfully ({event_count} event(s) emitted)."
                ))])
                .into())
            }
            ToolExecutionOutcome::QuerySuccess(val) => {
                if val.is_object() {
                    Ok(CallToolResult::structured(val).into())
                } else {
                    Ok(CallToolResult::success(vec![ContentBlock::text(
                        serde_json::to_string(&val).unwrap_or_default(),
                    )])
                    .into())
                }
            }
            ToolExecutionOutcome::DomainError(err) => {
                Ok(CallToolResult::error(vec![ContentBlock::text(err)]).into())
            }
            ToolExecutionOutcome::UnknownTool(name) => Err(ErrorData::invalid_params(
                format!("Unknown tool `{name}`"),
                None,
            )),
        }
    }
}

/// Handles a single MCP JSON-RPC request against `runtime`, returning `None` for notifications.
pub async fn handle_request(runtime: &DomainRuntime, req: &Value) -> Option<Value> {
    handle_request_with_actor(runtime, req, None).await
}

/// Handles a single MCP JSON-RPC request against `runtime` with an optional fallback actor
/// (such as from an HTTP `X-Actor-Type` / `X-Actor` header), returning `None` for notifications.
pub async fn handle_request_with_actor(
    runtime: &DomainRuntime,
    req: &Value,
    default_actor: Option<&str>,
) -> Option<Value> {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    match method {
        "initialize" => {
            let requested_version = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str)
                .unwrap_or(ProtocolVersion::LATEST_WITH_INITIALIZE.as_str());
            let negotiated = if ProtocolVersion::KNOWN_VERSIONS
                .iter()
                .any(|v| v.as_str() == requested_version && v.has_initialize())
            {
                requested_version
            } else {
                ProtocolVersion::LATEST_WITH_INITIALIZE.as_str()
            };
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": negotiated,
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": {
                        "name": runtime.domain_name(),
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }))
        }
        "server/discover" | "discover" => {
            let discover_result = rmcp::model::DiscoverResult::from_server_info(
                ProtocolVersion::KNOWN_VERSIONS.to_vec(),
                ServerConfig::new(
                    ServerCapabilities::builder()
                        .enable_tools()
                        .enable_resources()
                        .build(),
                )
                .with_server_info(Implementation::new(
                    runtime.domain_name(),
                    env!("CARGO_PKG_VERSION"),
                )),
            );
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": discover_result
            }))
        }
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": list_tools(runtime)
            }
        })),
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": list_resources(runtime)
            }
        })),
        "resources/templates/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resourceTemplates": list_resource_templates(runtime)
            }
        })),
        "resources/read" => {
            let uri = req
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let actor = req
                .get("params")
                .and_then(|p| p.get("_actor"))
                .and_then(Value::as_str)
                .or(default_actor);

            match read_esdm_resource(runtime, uri, actor).await {
                Ok(text) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "contents": [
                            {
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": text
                            }
                        ]
                    }
                })),
                Err(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": e
                    }
                })),
            }
        }
        "tools/call" => {
            let name = req
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let mut args = req
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            if let Some(actor_str) = default_actor
                && let Some(obj) = args.as_object_mut()
                && !obj.contains_key("_actor")
            {
                obj.insert("_actor".to_owned(), Value::String(actor_str.to_owned()));
            }

            match execute_tool_call(runtime, name, args).await {
                ToolExecutionOutcome::CommandSuccess(event_count) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": format!("Command executed successfully ({event_count} event(s) emitted).")
                            }
                        ]
                    }
                })),
                ToolExecutionOutcome::QuerySuccess(val) => {
                    let mut result_obj = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string(&val).unwrap_or_default()
                            }
                        ]
                    });
                    if val.is_object()
                        && let Some(map) = result_obj.as_object_mut()
                    {
                        map.insert("structuredContent".to_owned(), val);
                    }
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result_obj
                    }))
                }
                ToolExecutionOutcome::DomainError(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": e
                    }
                })),
                ToolExecutionOutcome::UnknownTool(name) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": format!("Unknown tool `{name}`")
                    }
                })),
            }
        }
        _ => {
            if method.starts_with("notifications/") {
                None
            } else {
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                }))
            }
        }
    }
}

/// Runs the official `rmcp` MCP stdio server loop over `stdin` / `stdout` for `runtime`.
pub async fn serve_mcp(runtime: Arc<DomainRuntime>) -> Result<(), anyhow::Error> {
    let service = McpDomainServer::new(runtime)
        .serve(rmcp::transport::stdio_pipes())
        .await?;
    service.waiting().await?;
    Ok(())
}

/// Entry-point builder for MCP presentation adapters over `stdio` or Streamable HTTP.
pub struct Mcp;

impl Mcp {
    /// Creates a standalone `stdio` MCP presentation adapter implementing
    /// [`chromatofoor::runtime::Presentation`] with zero HTTP dependencies.
    #[must_use]
    pub const fn stdio() -> McpStdio {
        McpStdio
    }

    /// Creates an MCP Streamable HTTP (`POST /mcp` + `GET /mcp` SSE) protocol adapter
    /// implementing [`chromatofoor_http::HttpProtocol`] so it can be attached to
    /// [`chromatofoor_http::HttpServer`].
    #[cfg(feature = "http")]
    #[must_use]
    pub fn http() -> McpHttp {
        McpHttp::default()
    }
}

/// MCP `stdio` presentation adapter implementing [`chromatofoor::runtime::Presentation`].
#[derive(Clone, Copy, Debug, Default)]
pub struct McpStdio;

impl chromatofoor::runtime::Presentation for McpStdio {
    fn name(&self) -> &'static str {
        "mcp-stdio"
    }

    async fn serve(
        self,
        runtime: Arc<DomainRuntime>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        serve_mcp(runtime).await?;
        Ok(())
    }
}

/// MCP Streamable HTTP (`POST` JSON-RPC + `GET` SSE) protocol adapter implementing
/// [`chromatofoor_http::HttpProtocol`].
#[cfg(feature = "http")]
#[derive(Clone, Debug)]
pub struct McpHttp {
    path: String,
}

#[cfg(feature = "http")]
impl Default for McpHttp {
    fn default() -> Self {
        Self {
            path: "/mcp".to_owned(),
        }
    }
}

#[cfg(feature = "http")]
impl McpHttp {
    /// Creates a new [`McpHttp`] adapter mounted at `"/mcp"`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Overrides the HTTP path where MCP Streamable HTTP (`POST` + `GET` SSE) is mounted.
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
}

#[cfg(feature = "http")]
impl chromatofoor_http::HttpProtocol for McpHttp {
    fn name(&self) -> &'static str {
        "mcp-http"
    }

    fn router(self, runtime: Arc<DomainRuntime>) -> axum::Router {
        routes_at(runtime, &self.path)
    }
}

/// Builds an [`axum::Router`] serving MCP Streamable HTTP (`POST /mcp` and `GET /mcp` SSE)
/// for `runtime`.
#[cfg(feature = "http")]
pub fn routes(runtime: Arc<DomainRuntime>) -> axum::Router {
    routes_at(runtime, "/mcp")
}

#[cfg(feature = "http")]
fn routes_at(runtime: Arc<DomainRuntime>, path: &str) -> axum::Router {
    use std::{collections::HashMap, convert::Infallible};

    use axum::{
        Json, Router,
        extract::{Query as AxumQuery, State},
        http::{HeaderMap, StatusCode},
        response::{
            IntoResponse,
            sse::{Event, Sse},
        },
        routing::get,
    };
    use tokio_stream::StreamExt as _;

    async fn mcp_post_handler(
        State(state): State<Arc<DomainRuntime>>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> axum::response::Response {
        let default_actor = chromatofoor_http::actor_from_headers(&headers);
        let prefers_sse = headers
            .get("accept")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|accept| {
                accept.contains("text/event-stream") && !accept.contains("application/json")
            });

        if let Some(batch) = body.as_array() {
            let mut responses = Vec::new();
            for item in batch {
                if let Some(resp) = handle_request_with_actor(&state, item, default_actor).await {
                    responses.push(resp);
                }
            }
            if responses.is_empty() {
                StatusCode::ACCEPTED.into_response()
            } else {
                Json(Value::Array(responses)).into_response()
            }
        } else {
            match handle_request_with_actor(&state, &body, default_actor).await {
                Some(resp) => {
                    if prefers_sse {
                        let data = serde_json::to_string(&resp).unwrap_or_default();
                        let stream = tokio_stream::once(Ok::<_, Infallible>(
                            Event::default().event("message").data(data),
                        ));
                        Sse::new(stream).into_response()
                    } else {
                        Json(resp).into_response()
                    }
                }
                None => StatusCode::ACCEPTED.into_response(),
            }
        }
    }

    async fn mcp_get_sse_handler(
        State(state): State<Arc<DomainRuntime>>,
        AxumQuery(filters): AxumQuery<HashMap<String, String>>,
    ) -> axum::response::Response {
        let subject_prefix = filters.get("subject_prefix").cloned();
        let event_type = filters.get("event_type").cloned();
        let rx = state.subscribe_events();
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
            .filter(move |event| {
                if let Some(prefix) = &subject_prefix
                    && !event.subject().starts_with(prefix)
                {
                    return false;
                }
                if let Some(ty) = &event_type
                    && event.ty() != ty
                {
                    return false;
                }
                true
            })
            .map(|event| {
                let notification = json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/domain_event",
                    "params": {
                        "id": event.id(),
                        "subject": event.subject(),
                        "type": event.ty(),
                        "data": event.data(),
                    }
                });
                let data = serde_json::to_string(&notification).unwrap_or_default();
                Ok::<_, Infallible>(Event::default().event("message").id(event.id()).data(data))
            });
        Sse::new(stream).into_response()
    }

    Router::new()
        .route(path, get(mcp_get_sse_handler).post(mcp_post_handler))
        .with_state(runtime)
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
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    use super::*;

    #[chromatofoor::domain]
    pub struct BankingDomain;

    #[chromatofoor::bounded_context(scope = BankingDomain)]
    pub struct Accounts;

    #[chromatofoor::actor(scope = Accounts, type = human)]
    pub struct Teller;

    #[chromatofoor::aggregate(scope = Accounts, identified_by = state(account_id))]
    #[derive(Clone, Default)]
    pub struct Account {
        pub account_id: Option<chromatofoor::Identifier>,
    }

    #[chromatofoor::event(scope = Account)]
    pub enum AccountEvent {
        AccountOpened {
            account_id: chromatofoor::Identifier,
        },
    }

    impl Event<Account> for AccountOpened {
        fn evolve(&self, _state: Account) -> Account {
            Account {
                account_id: Some(self.account_id.clone()),
            }
        }
    }

    #[chromatofoor::command(scope = Account)]
    pub enum AccountCommand {
        #[lifecycle(create)]
        #[actors(Teller)]
        OpenAccount {
            account_id: chromatofoor::Identifier,
        },
    }

    impl Command<Account> for OpenAccount {
        type Error = &'static str;

        async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
            Ok(vec![
                AccountOpened {
                    account_id: self.account_id,
                }
                .into(),
            ])
        }
    }

    #[chromatofoor::value_object(scope = Accounts)]
    pub struct AccountSummary {
        pub account_id: String,
    }

    #[chromatofoor::read_model(scope = Accounts, events = [AccountOpened])]
    pub struct AccountDirectory {
        pub accounts: InMemoryView<String, AccountSummary>,
    }

    impl Project<AccountOpened> for AccountDirectory {
        async fn project(&self, event: Envelope<AccountOpened>) -> Result<(), ViewError> {
            self.accounts.insert(
                &event,
                AccountSummary {
                    account_id: event.data.account_id.to_string(),
                },
            )?;
            Ok(())
        }
    }

    #[chromatofoor::query(scope = Accounts, actors = [Teller])]
    pub struct GetAccountSummary {
        pub account_id: String,
    }

    impl Query<AccountDirectory> for GetAccountSummary {
        type Output = AccountSummary;
        type Error = String;

        async fn execute(self, read_model: &AccountDirectory) -> Result<Self::Output, Self::Error> {
            read_model
                .accounts
                .get(&self.account_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("account `{}` not found", self.account_id))
        }
    }

    #[tokio::test]
    async fn catalog_resource_and_actor_tool_call_work() {
        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://bank.local").build());
        let directory = Arc::new(AccountDirectory {
            accounts: InMemoryView::new(|s: &AccountSummary| s.account_id.clone()),
        });
        let runtime = Arc::new(
            DomainBuilder::<BankingDomain>::new(store)
                .register_command::<OpenAccount, _>()
                .register_read_model(Arc::clone(&directory))
                .register_query::<GetAccountSummary, _>(Arc::clone(&directory))
                .build()
                .await
                .unwrap(),
        );

        // 0. Inspect `rmcp` tool annotations and `output_schema`
        let mcp_tools = build_mcp_tools(&runtime);
        assert_eq!(mcp_tools.len(), 2);
        let cmd_tool = mcp_tools
            .iter()
            .find(|t| t.name == "invoke_accounts_account_open-account")
            .unwrap();
        assert_eq!(
            cmd_tool.annotations.as_ref().and_then(|a| a.read_only_hint),
            Some(false)
        );
        assert_eq!(
            cmd_tool
                .annotations
                .as_ref()
                .and_then(|a| a.destructive_hint),
            Some(false)
        );

        let query_tool = mcp_tools
            .iter()
            .find(|t| t.name == "query_accounts_get-account-summary")
            .unwrap();
        assert_eq!(
            query_tool
                .annotations
                .as_ref()
                .and_then(|a| a.read_only_hint),
            Some(true)
        );
        assert_eq!(
            query_tool
                .annotations
                .as_ref()
                .and_then(|a| a.idempotent_hint),
            Some(true)
        );
        assert!(query_tool.output_schema.is_some());

        // 1. Read `esdm://catalog`
        let catalog_resp = handle_request(
            &runtime,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": { "uri": "esdm://catalog" }
            }),
        )
        .await
        .unwrap();
        let catalog_text = catalog_resp["result"]["contents"][0]["text"]
            .as_str()
            .unwrap();
        let catalog_json: Value = serde_json::from_str(catalog_text).unwrap();
        assert_eq!(catalog_json["domain"], "banking-domain");
        assert_eq!(
            catalog_json["commands"][0]["allowed_actors"],
            json!(["teller"])
        );

        // 2. Unauthorized `_actor` is rejected
        let unauthorized = handle_request(
            &runtime,
            &json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "invoke_accounts_account_open-account",
                    "arguments": {
                        "account_id": "acc-1",
                        "_actor": "customer"
                    }
                }
            }),
        )
        .await
        .unwrap();
        assert!(unauthorized.get("error").is_some());

        // 3. Authorized `_actor` succeeds
        let authorized = handle_request(
            &runtime,
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "invoke_accounts_account_open-account",
                    "arguments": {
                        "account_id": "acc-1",
                        "_actor": "teller"
                    }
                }
            }),
        )
        .await
        .unwrap();
        assert!(authorized.get("error").is_none(), "{authorized}");

        // 4. Query returns `structuredContent` for object outputs
        runtime.wait_caught_up().await.unwrap();
        let query_resp = handle_request(
            &runtime,
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": {
                    "name": "query_accounts_get-account-summary",
                    "arguments": {
                        "account_id": "acc-1",
                        "_actor": "teller"
                    }
                }
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            query_resp["result"]["structuredContent"]["account_id"],
            "acc-1"
        );

        // 5. Verify `McpDomainServer` over `rmcp` transport with `2026-07-28` `server/discover` and inline `tools/list`
        let (client_io, server_io) = tokio::io::duplex(8192);
        let server = McpDomainServer::new(Arc::clone(&runtime));
        let server_task = tokio::spawn(async move {
            let svc = server.serve(server_io).await.unwrap();
            let _ = svc.waiting().await;
        });

        let (client_read, mut client_write) = tokio::io::split(client_io);
        let mut client_lines = BufReader::new(client_read).lines();

        let discover_req = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "server/discover",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                    "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" }
                }
            }
        });
        client_write
            .write_all(format!("{discover_req}\n").as_bytes())
            .await
            .unwrap();
        client_write.flush().await.unwrap();

        let line = client_lines.next_line().await.unwrap().unwrap();
        let discover_resp: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(
            discover_resp["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
            "banking-domain"
        );
        assert!(
            discover_resp["result"]["supportedVersions"]
                .as_array()
                .unwrap()
                .contains(&json!("2026-07-28"))
        );

        let tools_list_req = json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/list",
            "params": {
                "_meta": {
                    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                    "io.modelcontextprotocol/clientCapabilities": {},
                    "io.modelcontextprotocol/clientInfo": { "name": "test-client", "version": "1.0.0" }
                }
            }
        });
        client_write
            .write_all(format!("{tools_list_req}\n").as_bytes())
            .await
            .unwrap();
        client_write.flush().await.unwrap();

        let tools_line = client_lines.next_line().await.unwrap().unwrap();
        let tools_resp: Value = serde_json::from_str(&tools_line).unwrap();
        assert_eq!(tools_resp["result"]["tools"].as_array().unwrap().len(), 2);

        client_write.shutdown().await.unwrap();
        drop(client_write);
        drop(client_lines);
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn mcp_stdio_attaches_as_domain_presentation() {
        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://bank.local").build());
        let runtime = DomainBuilder::<BankingDomain>::new(store)
            .register_command::<OpenAccount, _>()
            .attach(Mcp::stdio())
            .build()
            .await
            .unwrap();
        assert_eq!(runtime.attached_presentations(), vec!["mcp-stdio"]);
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn mcp_http_protocol_attaches_to_http_server() {
        use std::time::Duration;

        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use chromatofoor_http::HttpServer;
        use http_body_util::BodyExt as _;
        use tower::ServiceExt as _;

        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://bank.local").build());
        let directory = Arc::new(AccountDirectory {
            accounts: InMemoryView::new(|s: &AccountSummary| s.account_id.clone()),
        });
        let runtime = Arc::new(
            DomainBuilder::<BankingDomain>::new(store)
                .register_command::<OpenAccount, _>()
                .register_read_model(Arc::clone(&directory))
                .register_query::<GetAccountSummary, _>(Arc::clone(&directory))
                .build()
                .await
                .unwrap(),
        );

        let app = HttpServer::bind(3000)
            .attach(Mcp::http())
            .into_router(Arc::clone(&runtime));

        // 1. Open GET /mcp SSE stream with subject_prefix filter BEFORE dispatching command
        let sse_req = Request::builder()
            .method("GET")
            .uri("/mcp?subject_prefix=/banking-domain/accounts/account")
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
        let mut sse_body = sse_resp.into_body();

        // 2. POST /mcp tools/call using X-Actor-Type header
        let call_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("x-actor-type", "teller")
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "invoke_accounts_account_open-account",
                        "arguments": { "account_id": "acc-http-1" }
                    }
                })
                .to_string(),
            ))
            .unwrap();
        let resp = app.clone().oneshot(call_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let call_json: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(call_json.get("error").is_none(), "{call_json}");

        // 3. Notification returns 202 Accepted
        let notif_req = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized"
                })
                .to_string(),
            ))
            .unwrap();
        let notif_resp = app.clone().oneshot(notif_req).await.unwrap();
        assert_eq!(notif_resp.status(), StatusCode::ACCEPTED);

        // 4. Verify live `notifications/domain_event` frame on GET /mcp SSE stream
        let frame = tokio::time::timeout(Duration::from_secs(2), sse_body.frame())
            .await
            .expect("timed out waiting for MCP SSE frame")
            .expect("MCP SSE stream ended")
            .expect("MCP SSE frame error");
        let frame_text = String::from_utf8_lossy(frame.data_ref().unwrap());
        assert!(
            frame_text.contains("notifications/domain_event"),
            "{frame_text}"
        );
        assert!(frame_text.contains("acc-http-1"), "{frame_text}");
    }
}
