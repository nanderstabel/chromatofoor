//! Interactive terminal user interface (TUI) presentation adapter for
//! [`chromatofoor::domain::DomainRuntime`].
//!
//! Discovers actors, commands, queries, and JSON Schema templates directly from [`DomainRuntime`]
//! and streams live domain events in a split-pane terminal UI.
//!
//! # Example
//!
//! ```
//! use chromatofoor::architecturekit::{self, Schema};
//! use serde_json::json;
//!
//! #[derive(Schema)]
//! struct PlaceOrder {
//!     order_id: String,
//!     total_cents: u64,
//! }
//!
//! let template = chromatofoor_tui::generate_template(&PlaceOrder::schema());
//! let parsed: serde_json::Value = serde_json::from_str(&template).unwrap();
//! assert_eq!(parsed, json!({ "order_id": "...", "total_cents": 0 }));
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    collections::VecDeque,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use chromatofoor::{
    domain::DomainRuntime,
    runtime::{CommandDescriptor, Presentation, QueryDescriptor},
};
use crossterm::{
    event::{self, DisableBracketedPaste, EnableBracketedPaste, Event as CEvent, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use ratatui_textarea::TextArea;
use serde_json::Value;

/// Interactive terminal presentation adapter that can be attached to a [`DomainRuntime`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Tui;

impl Tui {
    /// Creates a new interactive terminal presentation adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Presentation for Tui {
    fn name(&self) -> &'static str {
        "tui"
    }

    async fn serve(
        self,
        runtime: Arc<DomainRuntime>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        run(runtime).await;
        Ok(())
    }
}

enum AppState {
    SelectActor(ListState, Vec<String>),
    Menu,
    CommandList(ListState, Vec<CommandDescriptor>),
    EnterCommandPayload {
        cmd: CommandDescriptor,
        textarea: TextArea<'static>,
    },
    QueryList(ListState, Vec<QueryDescriptor>),
    EnterQueryParams {
        query: QueryDescriptor,
        textarea: TextArea<'static>,
    },
    EnterEventFilter {
        textarea: TextArea<'static>,
    },
    ResultView(String),
}

fn resolve_schema<'a>(prop: &'a Value, root_schema: &'a Value) -> &'a Value {
    if let Some(ref_str) = prop.get("$ref").and_then(Value::as_str) {
        let def_name = ref_str
            .strip_prefix("#/$defs/")
            .or_else(|| ref_str.strip_prefix("#/definitions/"));
        if let Some(name) = def_name
            && let Some(resolved) = root_schema
                .get("$defs")
                .or_else(|| root_schema.get("definitions"))
                .and_then(|defs| defs.get(name))
        {
            return resolve_schema(resolved, root_schema);
        }
    }
    if let Some(all_of) = prop.get("allOf").and_then(Value::as_array)
        && all_of.len() == 1
    {
        return resolve_schema(&all_of[0], root_schema);
    }
    if let Some(branches) = prop
        .get("anyOf")
        .or_else(|| prop.get("oneOf"))
        .and_then(Value::as_array)
        && let Some(non_null) = branches
            .iter()
            .find(|b| b.get("type").and_then(Value::as_str) != Some("null"))
    {
        return resolve_schema(non_null, root_schema);
    }
    prop
}

fn effective_type(prop: &Value) -> &str {
    if let Some(ty) = prop.get("type") {
        if let Some(s) = ty.as_str() {
            return s;
        }
        if let Some(arr) = ty.as_array()
            && let Some(non_null) = arr.iter().filter_map(Value::as_str).find(|s| *s != "null")
        {
            return non_null;
        }
    }
    if prop.get("properties").is_some() {
        return "object";
    }
    "string"
}

fn generate_value(raw_prop: &Value, root_schema: &Value, depth: usize) -> Value {
    if depth > 8 {
        return serde_json::json!(null);
    }
    let prop = resolve_schema(raw_prop, root_schema);
    if let Some(default_val) = prop.get("default") {
        return default_val.clone();
    }
    if let Some(first_enum) = prop
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
    {
        return first_enum.clone();
    }

    match effective_type(prop) {
        "integer" | "number" => serde_json::json!(0),
        "boolean" => serde_json::json!(false),
        "array" => {
            if let Some(raw_items) = prop.get("items") {
                let items = resolve_schema(raw_items, root_schema);
                if effective_type(items) == "object" && items.get("properties").is_some() {
                    serde_json::json!([generate_value(items, root_schema, depth + 1)])
                } else {
                    serde_json::json!([])
                }
            } else {
                serde_json::json!([])
            }
        }
        "object" => {
            let mut map = serde_json::Map::new();
            if let Some(props) = prop.get("properties").and_then(Value::as_object) {
                for (k, child_prop) in props {
                    map.insert(
                        k.clone(),
                        generate_value(child_prop, root_schema, depth + 1),
                    );
                }
            }
            Value::Object(map)
        }
        _ => match prop.get("format").and_then(Value::as_str) {
            Some("uuid") => Value::String("00000000-0000-0000-0000-000000000001".to_owned()),
            Some("date") => Value::String("2026-01-01".to_owned()),
            Some("date-time") => Value::String("2026-01-01T00:00:00Z".to_owned()),
            _ => Value::String("...".to_owned()),
        },
    }
}

/// Generates a pretty-printed JSON skeleton from a JSON Schema object, recursively expanding
/// nested objects, `$defs` references, `enum` variants, and string `format` hints.
#[must_use]
pub fn generate_template(schema: &Value) -> String {
    let val = generate_value(schema, schema, 0);
    serde_json::to_string_pretty(&val).unwrap_or_else(|_| "{}".to_owned())
}

fn collect_actors(runtime: &DomainRuntime) -> Vec<String> {
    let actors = runtime.actors();
    if actors.is_empty() {
        vec!["anonymous".to_owned()]
    } else {
        actors.into_iter().map(str::to_owned).collect()
    }
}

fn filter_commands(runtime: &DomainRuntime, actor: &str) -> Vec<CommandDescriptor> {
    runtime
        .commands_for_actor(actor)
        .into_iter()
        .cloned()
        .collect()
}

fn filter_queries(runtime: &DomainRuntime, actor: &str) -> Vec<QueryDescriptor> {
    runtime
        .queries_for_actor(actor)
        .into_iter()
        .cloned()
        .collect()
}

fn textarea_from_schema(schema: &Value) -> TextArea<'static> {
    let template = generate_template(schema);
    let mut ta = TextArea::default();
    for line in template.lines() {
        ta.insert_str(line);
        ta.insert_newline();
    }
    ta
}

/// Runs the interactive terminal UI connected to `runtime`.
pub async fn run(runtime: Arc<DomainRuntime>) {
    let mut current_actor = String::new();
    let mut event_filter = String::new();
    let event_log = Arc::new(Mutex::new(VecDeque::new()));

    let mut rx = runtime.subscribe_events();
    let log_clone = Arc::clone(&event_log);
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let mut log = log_clone.lock().unwrap();
            let payload_str = serde_json::to_string(event.data()).unwrap_or_default();
            log.push_front(format!(
                "[{}] {} | payload: {}",
                event.subject(),
                event.ty(),
                payload_str
            ));
            if log.len() > 1000 {
                log.pop_back();
            }
        }
    });

    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let actor_types = collect_actors(&runtime);
    let mut ls = ListState::default();
    if !actor_types.is_empty() {
        ls.select(Some(0));
    }
    let mut state = AppState::SelectActor(ls, actor_types.clone());

    loop {
        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                    .split(f.area());

                let left_pane = chunks[0];
                let right_pane = chunks[1];

                let log = event_log.lock().unwrap();
                let items: Vec<ListItem> = log
                    .iter()
                    .filter(|msg| event_filter.is_empty() || msg.contains(&event_filter))
                    .map(|msg| ListItem::new(msg.as_str()))
                    .collect();
                let stream_title = if event_filter.is_empty() {
                    " Event Stream ".to_owned()
                } else {
                    format!(" Event Stream (filter: \"{event_filter}\") ")
                };
                let events_list = List::new(items)
                    .block(Block::default().title(stream_title).borders(Borders::ALL))
                    .style(Style::default().fg(Color::Green));
                f.render_widget(events_list, right_pane);

                match &mut state {
                    AppState::SelectActor(list_state, actors) => {
                        let items: Vec<ListItem> =
                            actors.iter().map(|a| ListItem::new(a.as_str())).collect();
                        let list = List::new(items)
                            .block(
                                Block::default()
                                    .title(" Select Actor (Enter to select, Esc to exit) ")
                                    .borders(Borders::ALL),
                            )
                            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                        f.render_stateful_widget(list, left_pane, list_state);
                    }
                    AppState::Menu => {
                        let filter_label = if event_filter.is_empty() {
                            "[f] Filter Event Stream".to_owned()
                        } else {
                            format!("[f] Filter Event Stream (active: \"{event_filter}\")")
                        };
                        let menu_text = [
                            format!("Domain: {} (Actor: {current_actor})", runtime.domain_name()),
                            String::new(),
                            "[c] Dispatch Command".to_owned(),
                            "[q] Execute Query".to_owned(),
                            "[a] Switch Actor".to_owned(),
                            filter_label,
                            "[x] Clear Event Stream".to_owned(),
                            "[ESC] Exit".to_owned(),
                            String::new(),
                            "Press a key to select an action.".to_owned(),
                        ];
                        let menu = Paragraph::new(menu_text.join("\n"))
                            .block(Block::default().title(" Menu ").borders(Borders::ALL))
                            .style(Style::default().fg(Color::Cyan));
                        f.render_widget(menu, left_pane);
                    }
                    AppState::CommandList(list_state, cmds) => {
                        let items: Vec<ListItem> = cmds
                            .iter()
                            .map(|c| {
                                ListItem::new(format!(
                                    "[{}] {} / {} / {} — {}",
                                    c.target_kind,
                                    c.bounded_context,
                                    c.target,
                                    c.name,
                                    c.description
                                ))
                            })
                            .collect();
                        let list = List::new(items)
                            .block(
                                Block::default()
                                    .title(" Select Command (Enter to select, Esc to go back) ")
                                    .borders(Borders::ALL),
                            )
                            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                        f.render_stateful_widget(list, left_pane, list_state);
                    }
                    AppState::EnterCommandPayload { cmd, textarea } => {
                        let title = format!(
                            " [{}] {} — {} (Ctrl+S submit, Esc back) ",
                            cmd.target_kind, cmd.name, cmd.description
                        );
                        textarea.set_block(Block::default().title(title).borders(Borders::ALL));
                        f.render_widget(&*textarea, left_pane);
                    }
                    AppState::QueryList(list_state, queries) => {
                        let items: Vec<ListItem> = queries
                            .iter()
                            .map(|q| {
                                ListItem::new(format!(
                                    "{} / {} — {}",
                                    q.bounded_context, q.name, q.description
                                ))
                            })
                            .collect();
                        let list = List::new(items)
                            .block(
                                Block::default()
                                    .title(" Select Query (Enter to select, Esc to go back) ")
                                    .borders(Borders::ALL),
                            )
                            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
                        f.render_stateful_widget(list, left_pane, list_state);
                    }
                    AppState::EnterQueryParams { query, textarea } => {
                        let title = format!(
                            " {} — {} (Ctrl+S submit, Esc back) ",
                            query.name, query.description
                        );
                        textarea.set_block(Block::default().title(title).borders(Borders::ALL));
                        f.render_widget(&*textarea, left_pane);
                    }
                    AppState::EnterEventFilter { textarea } => {
                        let title = " Event Stream Filter (Enter to apply, Esc to cancel) ";
                        textarea.set_block(Block::default().title(title).borders(Borders::ALL));
                        f.render_widget(&*textarea, left_pane);
                    }
                    AppState::ResultView(msg) => {
                        let p = Paragraph::new(msg.as_str())
                            .block(
                                Block::default()
                                    .title(" Result (Esc to go back) ")
                                    .borders(Borders::ALL),
                            )
                            .style(Style::default().fg(Color::Yellow));
                        f.render_widget(p, left_pane);
                    }
                }
            })
            .unwrap();

        if event::poll(Duration::from_millis(100)).unwrap() {
            let evt = event::read().unwrap();
            let mut next_state = None;

            if let CEvent::Key(key) = &evt {
                match &mut state {
                    AppState::SelectActor(list_state, actors) => match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Down => {
                            if let Some(i) = list_state.selected()
                                && i < actors.len().saturating_sub(1)
                            {
                                list_state.select(Some(i + 1));
                            }
                        }
                        KeyCode::Up => {
                            if let Some(i) = list_state.selected()
                                && i > 0
                            {
                                list_state.select(Some(i - 1));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(i) = list_state.selected()
                                && let Some(actor) = actors.get(i)
                            {
                                current_actor.clone_from(actor);
                                next_state = Some(AppState::Menu);
                            }
                        }
                        _ => {}
                    },
                    AppState::Menu => match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('a') => {
                            let mut ls = ListState::default();
                            if !actor_types.is_empty() {
                                ls.select(Some(0));
                            }
                            next_state = Some(AppState::SelectActor(ls, actor_types.clone()));
                        }
                        KeyCode::Char('c') => {
                            let cmds = filter_commands(&runtime, &current_actor);
                            let mut ls = ListState::default();
                            if !cmds.is_empty() {
                                ls.select(Some(0));
                            }
                            next_state = Some(AppState::CommandList(ls, cmds));
                        }
                        KeyCode::Char('q') => {
                            let queries = filter_queries(&runtime, &current_actor);
                            let mut ls = ListState::default();
                            if !queries.is_empty() {
                                ls.select(Some(0));
                            }
                            next_state = Some(AppState::QueryList(ls, queries));
                        }
                        KeyCode::Char('f') => {
                            let mut textarea = TextArea::default();
                            if !event_filter.is_empty() {
                                textarea.insert_str(&event_filter);
                            }
                            next_state = Some(AppState::EnterEventFilter { textarea });
                        }
                        KeyCode::Char('x') => {
                            event_log.lock().unwrap().clear();
                        }
                        _ => {}
                    },
                    AppState::CommandList(list_state, cmds) => match key.code {
                        KeyCode::Esc => next_state = Some(AppState::Menu),
                        KeyCode::Down => {
                            if let Some(i) = list_state.selected()
                                && i < cmds.len().saturating_sub(1)
                            {
                                list_state.select(Some(i + 1));
                            }
                        }
                        KeyCode::Up => {
                            if let Some(i) = list_state.selected()
                                && i > 0
                            {
                                list_state.select(Some(i - 1));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(i) = list_state.selected()
                                && let Some(cmd) = cmds.get(i).cloned()
                            {
                                let textarea = textarea_from_schema(&cmd.schema);
                                next_state = Some(AppState::EnterCommandPayload { cmd, textarea });
                            }
                        }
                        _ => {}
                    },
                    AppState::EnterCommandPayload { cmd, textarea } => {
                        if key.code == KeyCode::Char('s')
                            && key.modifiers.contains(event::KeyModifiers::CONTROL)
                        {
                            let json_str = textarea.lines().join("\n");
                            match serde_json::from_str::<Value>(&json_str) {
                                Ok(payload) => {
                                    let res = runtime
                                        .dispatch_command(
                                            cmd.bounded_context,
                                            cmd.target,
                                            cmd.name,
                                            payload,
                                            Some(&current_actor),
                                        )
                                        .await;
                                    match res {
                                        Ok(events) => {
                                            let _ = runtime.wait_caught_up().await;
                                            next_state = Some(AppState::ResultView(format!(
                                                "Command dispatched ({} event(s) emitted).",
                                                events.len()
                                            )));
                                        }
                                        Err(e) => {
                                            next_state = Some(AppState::ResultView(format!(
                                                "Command failed:\n{e}"
                                            )));
                                        }
                                    }
                                }
                                Err(e) => {
                                    next_state =
                                        Some(AppState::ResultView(format!("Invalid JSON:\n{e}")));
                                }
                            }
                        } else if key.code == KeyCode::Esc {
                            let cmds = filter_commands(&runtime, &current_actor);
                            let mut ls = ListState::default();
                            if !cmds.is_empty() {
                                ls.select(Some(0));
                            }
                            next_state = Some(AppState::CommandList(ls, cmds));
                        } else {
                            textarea.input(evt.clone());
                        }
                    }
                    AppState::QueryList(list_state, queries) => match key.code {
                        KeyCode::Esc => next_state = Some(AppState::Menu),
                        KeyCode::Down => {
                            if let Some(i) = list_state.selected()
                                && i < queries.len().saturating_sub(1)
                            {
                                list_state.select(Some(i + 1));
                            }
                        }
                        KeyCode::Up => {
                            if let Some(i) = list_state.selected()
                                && i > 0
                            {
                                list_state.select(Some(i - 1));
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(i) = list_state.selected()
                                && let Some(query) = queries.get(i).cloned()
                            {
                                let textarea = textarea_from_schema(&query.parameters_schema);
                                next_state = Some(AppState::EnterQueryParams { query, textarea });
                            }
                        }
                        _ => {}
                    },
                    AppState::EnterQueryParams { query, textarea } => {
                        if key.code == KeyCode::Char('s')
                            && key.modifiers.contains(event::KeyModifiers::CONTROL)
                        {
                            let json_str = textarea.lines().join("\n");
                            match serde_json::from_str::<Value>(&json_str) {
                                Ok(params) => {
                                    let res = runtime
                                        .dispatch_query(
                                            query.bounded_context,
                                            query.name,
                                            params,
                                            Some(&current_actor),
                                        )
                                        .await;
                                    match res {
                                        Ok(val) => {
                                            next_state = Some(AppState::ResultView(
                                                serde_json::to_string_pretty(&val)
                                                    .unwrap_or_default(),
                                            ));
                                        }
                                        Err(e) => {
                                            next_state = Some(AppState::ResultView(format!(
                                                "Query failed:\n{e}"
                                            )));
                                        }
                                    }
                                }
                                Err(e) => {
                                    next_state =
                                        Some(AppState::ResultView(format!("Invalid JSON:\n{e}")));
                                }
                            }
                        } else if key.code == KeyCode::Esc {
                            let queries = filter_queries(&runtime, &current_actor);
                            let mut ls = ListState::default();
                            if !queries.is_empty() {
                                ls.select(Some(0));
                            }
                            next_state = Some(AppState::QueryList(ls, queries));
                        } else {
                            textarea.input(evt.clone());
                        }
                    }
                    AppState::EnterEventFilter { textarea } => match key.code {
                        KeyCode::Enter => {
                            event_filter = textarea.lines().join("").trim().to_owned();
                            next_state = Some(AppState::Menu);
                        }
                        KeyCode::Esc => {
                            next_state = Some(AppState::Menu);
                        }
                        _ => {
                            textarea.input(evt.clone());
                        }
                    },
                    AppState::ResultView(_) => {
                        if key.code == KeyCode::Esc {
                            next_state = Some(AppState::Menu);
                        }
                    }
                }
            } else if let CEvent::Paste(text) = &evt {
                match &mut state {
                    AppState::EnterCommandPayload { textarea, .. }
                    | AppState::EnterQueryParams { textarea, .. }
                    | AppState::EnterEventFilter { textarea } => {
                        for line in text.lines() {
                            textarea.insert_str(line);
                            textarea.insert_newline();
                        }
                    }
                    _ => {}
                }
            }

            if let Some(s) = next_state {
                state = s;
            }
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )
    .unwrap();
    terminal.show_cursor().unwrap();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn generate_template_expands_nested_objects_enums_and_formats() {
        let schema = json!({
            "type": "object",
            "$defs": {
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "zip": { "type": "integer" }
                    }
                }
            },
            "properties": {
                "apartment_id": { "type": "string", "format": "uuid" },
                "move_in_date": { "type": "string", "format": "date" },
                "status": { "type": "string", "enum": ["pending", "active"] },
                "address": { "$ref": "#/$defs/Address" }
            }
        });

        let rendered = generate_template(&schema);
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            parsed,
            json!({
                "apartment_id": "00000000-0000-0000-0000-000000000001",
                "move_in_date": "2026-01-01",
                "status": "pending",
                "address": {
                    "street": "...",
                    "zip": 0
                }
            })
        );
    }
}
