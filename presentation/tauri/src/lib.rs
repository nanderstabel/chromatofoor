//! Tauri IPC presentation adapter for [`chromatofoor::domain::DomainRuntime`].
//!
//! Provides [`DomainState`] and `#[tauri::command]` handlers for dispatching commands, executing
//! queries, and inspecting the domain catalog from a Tauri frontend.
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use chromatofoor::{
//!     architecturekit::{InMemoryEventStore, Store},
//!     domain::DomainBuilder,
//! };
//! use chromatofoor_tauri::DomainState;
//!
//! #[chromatofoor::domain]
//! pub struct Commerce;
//!
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! let store = Arc::new(
//!     Store::builder(InMemoryEventStore::new(), "https://commerce.local").build(),
//! );
//! let runtime = Arc::new(DomainBuilder::<Commerce>::new(store).build().await.unwrap());
//! let state = DomainState {
//!     dispatcher: runtime,
//! };
//! assert_eq!(state.dispatcher.domain_name(), "commerce");
//! # });
//! ```

use std::sync::Arc;

use chromatofoor::domain::DomainRuntime;

/// Tauri managed state wrapping a shared [`DomainRuntime`].
pub struct DomainState {
    pub dispatcher: Arc<DomainRuntime>,
}

impl DomainState {
    /// Creates a new [`DomainState`] wrapping `dispatcher`.
    #[must_use]
    pub const fn new(dispatcher: Arc<DomainRuntime>) -> Self {
        Self { dispatcher }
    }
}

pub mod commands {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use chromatofoor::{
        ProblemDetails,
        architecturekit::{self, Envelope, Projection, ProjectionOptions, Subject, Subjects},
        domain::DomainRuntime,
    };
    use serde_json::{Value, json};

    use super::DomainState;

    fn serialize_event(event: &architecturekit::eventsourcingdb::Event) -> Value {
        let subject = event.subject();
        let instance_id = subject.rsplit('/').next().unwrap_or_default();
        let event_name = event.ty().rsplit('/').next().unwrap_or_else(|| event.ty());
        json!({
            "id": event.id(),
            "subject": subject,
            "instance_id": instance_id,
            "type": event.ty(),
            "event_name": event_name,
            "source": event.source(),
            "time": event.time().to_rfc3339(),
            "data": event.data(),
        })
    }

    fn resolve_command_actor<'a>(
        runtime: &'a DomainRuntime,
        ctx: &str,
        agg: &str,
        cmd_name: &str,
        explicit: Option<&'a str>,
    ) -> Option<&'a str> {
        if let Some(actor) = explicit.filter(|s| !s.is_empty()) {
            return Some(actor);
        }
        runtime
            .commands()
            .iter()
            .find(|c| c.bounded_context == ctx && c.target == agg && c.name == cmd_name)
            .and_then(|c| c.allowed_actors.first().map(|(actor, _)| *actor))
            .or_else(|| runtime.actors().first().copied())
    }

    fn resolve_query_actor<'a>(
        runtime: &'a DomainRuntime,
        ctx: &str,
        query_name: &str,
        explicit: Option<&'a str>,
    ) -> Option<&'a str> {
        if let Some(actor) = explicit.filter(|s| !s.is_empty()) {
            return Some(actor);
        }
        runtime
            .queries()
            .iter()
            .find(|q| q.bounded_context == ctx && q.name == query_name)
            .and_then(|q| q.allowed_actors.first().map(|(actor, _)| *actor))
            .or_else(|| runtime.actors().first().copied())
    }

    struct EventCollector {
        events: Mutex<Vec<Value>>,
    }

    impl Projection for EventCollector {
        type Event = architecturekit::eventsourcingdb::Event;
        type Error = std::convert::Infallible;

        async fn apply(&self, event: Envelope<Self::Event>) -> Result<(), Self::Error> {
            if let Ok(mut guard) = self.events.lock() {
                guard.push(serialize_event(&event.data));
            }
            Ok(())
        }
    }

    #[tauri::command]
    pub async fn dispatch_command(
        ctx: String,
        agg: String,
        cmd_name: String,
        payload: Value,
        metadata: HashMap<String, String>,
        state: tauri::State<'_, DomainState>,
    ) -> Result<Value, ProblemDetails> {
        let explicit_actor = metadata
            .get("actor_type")
            .or_else(|| metadata.get("actor"))
            .map(String::as_str);
        let actor = resolve_command_actor(&state.dispatcher, &ctx, &agg, &cmd_name, explicit_actor);
        let events = state
            .dispatcher
            .dispatch_command(&ctx, &agg, &cmd_name, payload, actor)
            .await
            .map_err(|e| e.to_problem())?;
        let _ = state.dispatcher.wait_caught_up().await;
        let serialized: Vec<Value> = events.iter().map(serialize_event).collect();
        Ok(json!({
            "events": serialized,
        }))
    }

    #[tauri::command]
    pub async fn query_read_model(
        ctx: String,
        query_name: String,
        params: Option<Value>,
        metadata: Option<HashMap<String, String>>,
        state: tauri::State<'_, DomainState>,
    ) -> Result<Value, ProblemDetails> {
        let explicit_actor = metadata
            .as_ref()
            .and_then(|m| m.get("actor_type").or_else(|| m.get("actor")))
            .map(String::as_str);
        let actor = resolve_query_actor(&state.dispatcher, &ctx, &query_name, explicit_actor);
        state
            .dispatcher
            .dispatch_query(
                &ctx,
                &query_name,
                params.unwrap_or_else(|| json!({})),
                actor,
            )
            .await
            .map_err(|e| e.to_problem())
    }

    #[tauri::command]
    pub async fn execute_command(
        command_name: String,
        payload: Value,
        metadata: Option<HashMap<String, String>>,
        state: tauri::State<'_, DomainState>,
    ) -> Result<Value, ProblemDetails> {
        let parts: Vec<&str> = command_name.split('.').collect();
        let (ctx, agg, cmd_name) = match parts.as_slice() {
            [c, a, m] => (*c, *a, *m),
            [c, m] => {
                let target = state
                    .dispatcher
                    .commands()
                    .iter()
                    .find(|cmd| cmd.bounded_context == *c && cmd.name == *m)
                    .map_or(*c, |cmd| cmd.target);
                (*c, target, *m)
            }
            _ => {
                return Err(ProblemDetails::bad_request(format!(
                    "Invalid command name format: '{command_name}'. Expected 'context.target.command'"
                )));
            }
        };

        let explicit_actor = metadata
            .as_ref()
            .and_then(|m| m.get("actor_type").or_else(|| m.get("actor")))
            .map(String::as_str);
        let actor = resolve_command_actor(&state.dispatcher, ctx, agg, cmd_name, explicit_actor);
        let events = state
            .dispatcher
            .dispatch_command(ctx, agg, cmd_name, payload, actor)
            .await
            .map_err(|e| e.to_problem())?;
        let _ = state.dispatcher.wait_caught_up().await;
        let serialized: Vec<Value> = events.iter().map(serialize_event).collect();
        Ok(json!({
            "events": serialized,
        }))
    }

    #[tauri::command]
    pub async fn execute_query(
        query_name: String,
        params: Value,
        metadata: Option<HashMap<String, String>>,
        state: tauri::State<'_, DomainState>,
    ) -> Result<Value, ProblemDetails> {
        let parts: Vec<&str> = query_name.split('.').collect();
        let (ctx, q_name) = match parts.as_slice() {
            [c, q] => (*c, *q),
            _ => {
                return Err(ProblemDetails::bad_request(format!(
                    "Invalid query name format: '{query_name}'. Expected 'context.query'"
                )));
            }
        };

        let explicit_actor = metadata
            .as_ref()
            .and_then(|m| m.get("actor_type").or_else(|| m.get("actor")))
            .map(String::as_str);
        let actor = resolve_query_actor(&state.dispatcher, ctx, q_name, explicit_actor);
        state
            .dispatcher
            .dispatch_query(ctx, q_name, params, actor)
            .await
            .map_err(|e| e.to_problem())
    }

    #[tauri::command]
    pub async fn get_schema(state: tauri::State<'_, DomainState>) -> Result<Value, String> {
        Ok(state.dispatcher.catalog_json())
    }

    #[tauri::command]
    pub async fn get_events(state: tauri::State<'_, DomainState>) -> Result<Vec<Value>, String> {
        let collector = Arc::new(EventCollector {
            events: Mutex::new(Vec::new()),
        });
        let token = architecturekit::CancellationToken::new();
        let topic = state.dispatcher.domain_topic();
        let run = state.dispatcher.store().start_projection(
            Subjects::tree(Subject::new(topic)),
            Arc::clone(&collector),
            token.clone(),
            ProjectionOptions::named(format!("{topic}-tauri-events")),
        );
        architecturekit::wait_caught_up(&[&run])
            .await
            .map_err(|e| e.to_string())?;
        token.cancel();
        let events = collector
            .events
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default();
        Ok(events)
    }

    #[tauri::command]
    #[allow(clippy::needless_pass_by_value)]
    pub fn window_control(
        #[allow(unused_variables)] window: tauri::WebviewWindow,
        action: String,
    ) -> Result<bool, String> {
        match action.as_str() {
            "drag" => {
                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                window.start_dragging().map_err(|e| e.to_string())?;
                Ok(false)
            }
            "minimize" => {
                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                window.minimize().map_err(|e| e.to_string())?;
                Ok(false)
            }
            "toggle_maximize" => {
                #[cfg(not(any(target_os = "android", target_os = "ios")))]
                {
                    let is_max = window.is_maximized().map_err(|e| e.to_string())?;
                    if is_max {
                        window.unmaximize().map_err(|e| e.to_string())?;
                        return Ok(false);
                    }
                    window.maximize().map_err(|e| e.to_string())?;
                    Ok(true)
                }
                #[cfg(any(target_os = "android", target_os = "ios"))]
                Ok(false)
            }
            "close" => {
                window.close().map_err(|e| e.to_string())?;
                Ok(false)
            }
            _ => Err(format!("Unknown window action: {action}")),
        }
    }
}
