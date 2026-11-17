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
    architecturekit::eventsourcingdb::Event as StoredEvent,
    domain::DomainRuntime,
    runtime::{CommandDescriptor, Presentation, QueryDescriptor},
};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event as CEvent, KeyCode, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultKind {
    Success,
    Error,
}

#[derive(Debug, Clone)]
struct EventEntry {
    subject: String,
    event_type: String,
    payload_pretty: String,
    payload_compact: String,
}

impl EventEntry {
    fn from_stored(event: &StoredEvent) -> Self {
        let payload_compact = serde_json::to_string(event.data()).unwrap_or_default();
        let payload_pretty =
            serde_json::to_string_pretty(event.data()).unwrap_or_else(|_| payload_compact.clone());
        Self {
            subject: event.subject().to_string(),
            event_type: event.ty().to_owned(),
            payload_pretty,
            payload_compact,
        }
    }

    fn matches_filter(&self, filter_lower: &str) -> bool {
        if filter_lower.is_empty() {
            return true;
        }
        self.subject.to_lowercase().contains(filter_lower)
            || self.event_type.to_lowercase().contains(filter_lower)
            || self.payload_compact.to_lowercase().contains(filter_lower)
    }

    fn detail_body(&self) -> String {
        format!(
            "Type:    {}\nSubject: {}\n\nPayload:\n{}",
            self.event_type, self.subject, self.payload_pretty
        )
    }
}

enum AppState {
    SelectActor(ListState, Vec<String>),
    Menu,
    EventStream,
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
        return_to_stream: bool,
    },
    ResultView {
        kind: ResultKind,
        title: String,
        body: String,
        scroll: u16,
    },
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let width = max_width.max(8);
    let mut out = Vec::new();

    for raw_line in text.split('\n') {
        if raw_line.is_empty() {
            out.push(String::new());
            continue;
        }

        let leading_spaces = raw_line.chars().take_while(|c| *c == ' ').count();
        let indent = " ".repeat(leading_spaces.min(width / 2));
        let mut current = indent.clone();

        for word in raw_line.split_whitespace() {
            let word_len = word.chars().count();
            let cur_len = current.chars().count();
            let sep = usize::from(cur_len > indent.len());

            if cur_len + sep + word_len <= width {
                if sep == 1 {
                    current.push(' ');
                }
                current.push_str(word);
            } else {
                if cur_len > indent.len() {
                    out.push(current);
                    current = indent.clone();
                }
                if indent.len() + word_len <= width {
                    current.push_str(word);
                } else {
                    let chunk_cap = width.saturating_sub(indent.len()).max(1);
                    let mut chars = word.chars().peekable();
                    while chars.peek().is_some() {
                        let chunk: String = chars.by_ref().take(chunk_cap).collect();
                        if chars.peek().is_some() {
                            out.push(format!("{indent}{chunk}"));
                        } else {
                            current = format!("{indent}{chunk}");
                        }
                    }
                }
            }
        }

        if !current.is_empty() {
            out.push(current);
        }
    }

    if out.is_empty() {
        out.push(String::new());
    }
    out
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
    let lines: Vec<&str> = template.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        ta.insert_str(line);
        if idx + 1 < lines.len() {
            ta.insert_newline();
        }
    }
    ta
}

fn format_command_success(cmd_name: &str, events: &[StoredEvent]) -> String {
    if events.is_empty() {
        return format!("Command '{cmd_name}' succeeded (0 events emitted).");
    }

    let mut sections = vec![format!(
        "Command '{cmd_name}' succeeded — {} event(s) emitted:",
        events.len()
    )];
    for (idx, ev) in events.iter().enumerate() {
        let payload = serde_json::to_string_pretty(ev.data()).unwrap_or_default();
        sections.push(format!(
            "\n#{} {}\nSubject: {}\nPayload:\n{}",
            idx + 1,
            ev.ty(),
            ev.subject(),
            payload
        ));
    }
    sections.join("\n")
}

fn build_wrapped_list_item(
    header: String,
    description: &str,
    inner_width: usize,
) -> ListItem<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    for (idx, wrapped) in wrap_text(&header, inner_width).into_iter().enumerate() {
        let prefix = if idx == 0 { "" } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{prefix}{wrapped}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if !description.trim().is_empty() {
        let desc_width = inner_width.saturating_sub(2);
        for wrapped in wrap_text(description.trim(), desc_width) {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped}"),
                Style::default().fg(Color::Gray),
            )));
        }
    }
    ListItem::new(Text::from(lines))
}

fn build_event_list_item(entry: &EventEntry, inner_width: usize) -> ListItem<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let header = format!("{}  [{}]", entry.event_type, entry.subject);
    for (idx, wrapped) in wrap_text(&header, inner_width).into_iter().enumerate() {
        if idx == 0 {
            if let Some((ev_ty, rest)) = wrapped.split_once("  ") {
                lines.push(Line::from(vec![
                    Span::styled(
                        ev_ty.to_owned(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(rest.to_owned(), Style::default().fg(Color::Cyan)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
            }
        } else {
            lines.push(Line::from(Span::styled(
                format!("  {wrapped}"),
                Style::default().fg(Color::Cyan),
            )));
        }
    }

    let payload_width = inner_width.saturating_sub(2);
    let payload_line = format!("payload: {}", entry.payload_compact);
    for wrapped in wrap_text(&payload_line, payload_width) {
        lines.push(Line::from(Span::styled(
            format!("  {wrapped}"),
            Style::default().fg(Color::Gray),
        )));
    }

    ListItem::new(Text::from(lines))
}

fn split_header_and_editor(area: Rect, header_text: &str) -> (Rect, Rect) {
    let inner_width = usize::from(area.width.saturating_sub(2)).max(8);
    let wrapped_count = wrap_text(header_text, inner_width).len() as u16;
    let header_height = (wrapped_count + 2).clamp(4, area.height.saturating_sub(5).max(4));
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(3)])
        .split(area);
    (parts[0], parts[1])
}

fn render_ui(
    f: &mut Frame<'_>,
    domain_name: &str,
    current_actor: &str,
    event_filter: &str,
    log: &VecDeque<EventEntry>,
    event_list_state: &mut ListState,
    state: &mut AppState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(f.area());

    let left_pane = chunks[0];
    let right_pane = chunks[1];

    let left_inner_width = usize::from(left_pane.width.saturating_sub(4)).max(8);
    let right_inner_width = usize::from(right_pane.width.saturating_sub(4)).max(8);

    let filter_lower = event_filter.to_lowercase();
    let filtered_entries: Vec<&EventEntry> = log
        .iter()
        .filter(|entry| entry.matches_filter(&filter_lower))
        .collect();

    if filtered_entries.is_empty() {
        event_list_state.select(None);
    } else if let Some(sel) = event_list_state.selected() {
        if sel >= filtered_entries.len() {
            event_list_state.select(Some(filtered_entries.len() - 1));
        }
    } else if matches!(state, AppState::EventStream) {
        event_list_state.select(Some(0));
    }

    let event_items: Vec<ListItem> = filtered_entries
        .iter()
        .map(|entry| build_event_list_item(entry, right_inner_width))
        .collect();

    let stream_focused = matches!(state, AppState::EventStream);
    let count_badge = if event_filter.is_empty() {
        format!("{}", filtered_entries.len())
    } else {
        format!("{}/{}", filtered_entries.len(), log.len())
    };
    let stream_title = match (stream_focused, event_filter.is_empty()) {
        (true, true) => {
            format!(" Event Stream [{count_badge}] (Enter inspect, f filter, x clear, Esc back) ")
        }
        (true, false) => format!(
            " Event Stream [{count_badge}] filter=\"{event_filter}\" (Enter inspect, r reset, Esc back) "
        ),
        (false, true) => format!(" Event Stream [{count_badge}] ([e] inspect, [f] filter) "),
        (false, false) => {
            format!(" Event Stream [{count_badge}] (filter: \"{event_filter}\", [r] reset) ")
        }
    };
    let stream_border_style = if stream_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let events_list = List::new(event_items)
        .block(
            Block::default()
                .title(stream_title)
                .borders(Borders::ALL)
                .border_style(stream_border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(events_list, right_pane, event_list_state);

    match state {
        AppState::SelectActor(list_state, actors) => {
            let items: Vec<ListItem> = actors.iter().map(|a| ListItem::new(a.as_str())).collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Select Actor (Enter select, Esc exit) ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            f.render_stateful_widget(list, left_pane, list_state);
        }
        AppState::Menu => {
            let filter_label = if event_filter.is_empty() {
                "[f] Filter Event Stream".to_owned()
            } else {
                format!("[f] Edit Event Filter (active: \"{event_filter}\")")
            };
            let mut menu_lines = vec![
                format!("Domain: {domain_name}"),
                format!("Actor:  {current_actor}"),
                String::new(),
                "[c] Dispatch Command".to_owned(),
                "[q] Execute Query".to_owned(),
                "[e] Inspect / Scroll Event Stream (Tab)".to_owned(),
                "[a] Switch Actor".to_owned(),
                filter_label,
            ];
            if !event_filter.is_empty() {
                menu_lines.push("[r] Reset Event Filter".to_owned());
            }
            menu_lines.push("[x] Clear Event Stream".to_owned());
            menu_lines.push("[Esc] Exit".to_owned());
            menu_lines.push(String::new());
            menu_lines.push("Press a shortcut key to continue.".to_owned());

            let menu = Paragraph::new(menu_lines.join("\n"))
                .block(
                    Block::default()
                        .title(" Menu ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: false });
            f.render_widget(menu, left_pane);
        }
        AppState::EventStream => {
            let mut help_lines = vec![
                format!("Domain: {domain_name}"),
                format!("Actor:  {current_actor}"),
                String::new(),
                "Event Stream Focus Active:".to_owned(),
                "  [Up/Down]   Select event".to_owned(),
                "  [PgUp/PgDn] Jump 10 events".to_owned(),
                "  [Home]      Newest event".to_owned(),
                "  [Enter]     Inspect full event payload".to_owned(),
                "  [f]         Filter events (case-insensitive)".to_owned(),
            ];
            if !event_filter.is_empty() {
                help_lines.push(format!(
                    "  [r]         Reset active filter (\"{event_filter}\")"
                ));
            }
            help_lines.push("  [x]         Clear event stream".to_owned());
            help_lines.push("  [Esc/Tab]   Return to Menu".to_owned());

            let panel = Paragraph::new(help_lines.join("\n"))
                .block(
                    Block::default()
                        .title(" Event Stream Controls ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false });
            f.render_widget(panel, left_pane);
        }
        AppState::CommandList(list_state, cmds) => {
            let items: Vec<ListItem> = cmds
                .iter()
                .map(|c| {
                    let header = format!(
                        "[{}] {} / {} / {}",
                        c.target_kind, c.bounded_context, c.target, c.name
                    );
                    build_wrapped_list_item(header, c.description, left_inner_width)
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Select Command (Enter select, Esc back) ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            f.render_stateful_widget(list, left_pane, list_state);
        }
        AppState::EnterCommandPayload { cmd, textarea } => {
            let header_body = format!(
                "[{}] {} / {} / {}\n{}\nCtrl+S: Submit  |  Esc: Back",
                cmd.target_kind, cmd.bounded_context, cmd.target, cmd.name, cmd.description
            );
            let (header_area, editor_area) = split_header_and_editor(left_pane, &header_body);
            let header = Paragraph::new(header_body)
                .block(
                    Block::default()
                        .title(" Command ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: false });
            f.render_widget(header, header_area);

            textarea.set_block(
                Block::default()
                    .title(" Payload JSON (Ctrl+S submit, Esc back) ")
                    .borders(Borders::ALL),
            );
            f.render_widget(&*textarea, editor_area);
        }
        AppState::QueryList(list_state, queries) => {
            let items: Vec<ListItem> = queries
                .iter()
                .map(|q| {
                    let header = format!("{} / {}", q.bounded_context, q.name);
                    build_wrapped_list_item(header, q.description, left_inner_width)
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Select Query (Enter select, Esc back) ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            f.render_stateful_widget(list, left_pane, list_state);
        }
        AppState::EnterQueryParams { query, textarea } => {
            let header_body = format!(
                "{} / {} (read-model: {})\n{}\nCtrl+S: Submit  |  Esc: Back",
                query.bounded_context, query.name, query.read_model, query.description
            );
            let (header_area, editor_area) = split_header_and_editor(left_pane, &header_body);
            let header = Paragraph::new(header_body)
                .block(
                    Block::default()
                        .title(" Query ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .style(Style::default().fg(Color::Cyan))
                .wrap(Wrap { trim: false });
            f.render_widget(header, header_area);

            textarea.set_block(
                Block::default()
                    .title(" Parameters JSON (Ctrl+S submit, Esc back) ")
                    .borders(Borders::ALL),
            );
            f.render_widget(&*textarea, editor_area);
        }
        AppState::EnterEventFilter { textarea, .. } => {
            let header_body = "Case-insensitive substring match across subject, event type, and payload.\nEnter / Ctrl+S: Apply  |  Ctrl+U: Clear  |  Esc: Cancel";
            let (header_area, editor_area) = split_header_and_editor(left_pane, header_body);
            let header = Paragraph::new(header_body)
                .block(
                    Block::default()
                        .title(" Filter Event Stream ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: false });
            f.render_widget(header, header_area);

            textarea.set_block(
                Block::default()
                    .title(" Filter Text (empty clears filter) ")
                    .borders(Borders::ALL),
            );
            f.render_widget(&*textarea, editor_area);
        }
        AppState::ResultView {
            kind,
            title,
            body,
            scroll,
        } => {
            let (border_style, text_style, badge) = match kind {
                ResultKind::Success => (
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::White),
                    "OK",
                ),
                ResultKind::Error => (
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::LightRed),
                    "ERROR",
                ),
            };
            let block_title = format!(" [{badge}] {title} (Up/Down scroll, Esc back) ");
            let p = Paragraph::new(body.as_str())
                .block(
                    Block::default()
                        .title(block_title)
                        .borders(Borders::ALL)
                        .border_style(border_style),
                )
                .style(text_style)
                .wrap(Wrap { trim: false })
                .scroll((*scroll, 0));
            f.render_widget(p, left_pane);
        }
    }
}

fn open_filter_editor(current_filter: &str, return_to_stream: bool) -> AppState {
    let mut textarea = TextArea::default();
    if !current_filter.is_empty() {
        textarea.insert_str(current_filter);
    }
    AppState::EnterEventFilter {
        textarea,
        return_to_stream,
    }
}

/// Runs the interactive terminal UI connected to `runtime`.
pub async fn run(runtime: Arc<DomainRuntime>) {
    let mut current_actor = String::new();
    let mut event_filter = String::new();
    let event_log: Arc<Mutex<VecDeque<EventEntry>>> = Arc::new(Mutex::new(VecDeque::new()));
    let mut event_list_state = ListState::default();

    let mut rx = runtime.subscribe_events();
    let log_clone = Arc::clone(&event_log);
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let mut log = log_clone.lock().unwrap();
            log.push_front(EventEntry::from_stored(&event));
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
                let log = event_log.lock().unwrap();
                render_ui(
                    f,
                    runtime.domain_name(),
                    &current_actor,
                    &event_filter,
                    &log,
                    &mut event_list_state,
                    &mut state,
                );
            })
            .unwrap();

        if event::poll(Duration::from_millis(100)).unwrap() {
            let evt = event::read().unwrap();
            let mut next_state = None;

            if let CEvent::Key(key) = &evt {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

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
                        KeyCode::Char('e') | KeyCode::Tab => {
                            if event_list_state.selected().is_none() {
                                event_list_state.select(Some(0));
                            }
                            next_state = Some(AppState::EventStream);
                        }
                        KeyCode::Char('f') => {
                            next_state = Some(open_filter_editor(&event_filter, false));
                        }
                        KeyCode::Char('r') => {
                            event_filter.clear();
                        }
                        KeyCode::Char('x') => {
                            event_log.lock().unwrap().clear();
                            event_list_state.select(None);
                        }
                        _ => {}
                    },
                    AppState::EventStream => {
                        let filtered_len = {
                            let log = event_log.lock().unwrap();
                            let filter_lower = event_filter.to_lowercase();
                            log.iter()
                                .filter(|entry| entry.matches_filter(&filter_lower))
                                .count()
                        };
                        match key.code {
                            KeyCode::Esc | KeyCode::Tab => {
                                next_state = Some(AppState::Menu);
                            }
                            KeyCode::Down | KeyCode::Char('j') if filtered_len > 0 => {
                                let i = event_list_state.selected().unwrap_or(0);
                                if i < filtered_len - 1 {
                                    event_list_state.select(Some(i + 1));
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if let Some(i) = event_list_state.selected()
                                    && i > 0
                                {
                                    event_list_state.select(Some(i - 1));
                                }
                            }
                            KeyCode::PageDown if filtered_len > 0 => {
                                let i = event_list_state.selected().unwrap_or(0);
                                event_list_state.select(Some((i + 10).min(filtered_len - 1)));
                            }
                            KeyCode::PageUp => {
                                if let Some(i) = event_list_state.selected() {
                                    event_list_state.select(Some(i.saturating_sub(10)));
                                }
                            }
                            KeyCode::Home if filtered_len > 0 => {
                                event_list_state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                if let Some(i) = event_list_state.selected() {
                                    let detail = {
                                        let log = event_log.lock().unwrap();
                                        let filter_lower = event_filter.to_lowercase();
                                        log.iter()
                                            .filter(|e| e.matches_filter(&filter_lower))
                                            .nth(i)
                                            .cloned()
                                    };
                                    if let Some(entry) = detail {
                                        next_state = Some(AppState::ResultView {
                                            kind: ResultKind::Success,
                                            title: format!("Event: {}", entry.event_type),
                                            body: entry.detail_body(),
                                            scroll: 0,
                                        });
                                    }
                                }
                            }
                            KeyCode::Char('f') => {
                                next_state = Some(open_filter_editor(&event_filter, true));
                            }
                            KeyCode::Char('r') => {
                                event_filter.clear();
                                event_list_state.select(Some(0));
                            }
                            KeyCode::Char('x') => {
                                event_log.lock().unwrap().clear();
                                event_list_state.select(None);
                            }
                            _ => {}
                        }
                    }
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
                        KeyCode::Char('f') => {
                            next_state = Some(open_filter_editor(&event_filter, false));
                        }
                        KeyCode::Char('r') => {
                            event_filter.clear();
                        }
                        KeyCode::Char('x') => {
                            event_log.lock().unwrap().clear();
                            event_list_state.select(None);
                        }
                        _ => {}
                    },
                    AppState::EnterCommandPayload { cmd, textarea } => {
                        if key.code == KeyCode::Char('s') && is_ctrl {
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
                                            next_state = Some(AppState::ResultView {
                                                kind: ResultKind::Success,
                                                title: format!("Command: {}", cmd.name),
                                                body: format_command_success(cmd.name, &events),
                                                scroll: 0,
                                            });
                                        }
                                        Err(e) => {
                                            next_state = Some(AppState::ResultView {
                                                kind: ResultKind::Error,
                                                title: format!("Command Failed: {}", cmd.name),
                                                body: format!("{e}"),
                                                scroll: 0,
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    next_state = Some(AppState::ResultView {
                                        kind: ResultKind::Error,
                                        title: "Invalid Command JSON".to_owned(),
                                        body: format!("{e}"),
                                        scroll: 0,
                                    });
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
                        KeyCode::Char('f') => {
                            next_state = Some(open_filter_editor(&event_filter, false));
                        }
                        KeyCode::Char('r') => {
                            event_filter.clear();
                        }
                        KeyCode::Char('x') => {
                            event_log.lock().unwrap().clear();
                            event_list_state.select(None);
                        }
                        _ => {}
                    },
                    AppState::EnterQueryParams { query, textarea } => {
                        if key.code == KeyCode::Char('s') && is_ctrl {
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
                                            next_state = Some(AppState::ResultView {
                                                kind: ResultKind::Success,
                                                title: format!("Query: {}", query.name),
                                                body: serde_json::to_string_pretty(&val)
                                                    .unwrap_or_default(),
                                                scroll: 0,
                                            });
                                        }
                                        Err(e) => {
                                            next_state = Some(AppState::ResultView {
                                                kind: ResultKind::Error,
                                                title: format!("Query Failed: {}", query.name),
                                                body: format!("{e}"),
                                                scroll: 0,
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    next_state = Some(AppState::ResultView {
                                        kind: ResultKind::Error,
                                        title: "Invalid Query JSON".to_owned(),
                                        body: format!("{e}"),
                                        scroll: 0,
                                    });
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
                    AppState::EnterEventFilter {
                        textarea,
                        return_to_stream,
                    } => {
                        let back_state = || {
                            if *return_to_stream {
                                AppState::EventStream
                            } else {
                                AppState::Menu
                            }
                        };
                        if key.code == KeyCode::Enter || (key.code == KeyCode::Char('s') && is_ctrl)
                        {
                            event_filter = textarea.lines().join(" ").trim().to_owned();
                            next_state = Some(back_state());
                        } else if key.code == KeyCode::Char('u') && is_ctrl {
                            *textarea = TextArea::default();
                        } else if key.code == KeyCode::Esc {
                            next_state = Some(back_state());
                        } else {
                            textarea.input(evt.clone());
                        }
                    }
                    AppState::ResultView { scroll, .. } => match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            next_state = Some(AppState::Menu);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            *scroll = scroll.saturating_add(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            *scroll = scroll.saturating_sub(1);
                        }
                        KeyCode::PageDown => {
                            *scroll = scroll.saturating_add(10);
                        }
                        KeyCode::PageUp => {
                            *scroll = scroll.saturating_sub(10);
                        }
                        KeyCode::Home => {
                            *scroll = 0;
                        }
                        KeyCode::Char('f') => {
                            next_state = Some(open_filter_editor(&event_filter, false));
                        }
                        KeyCode::Char('r') => {
                            event_filter.clear();
                        }
                        KeyCode::Char('x') => {
                            event_log.lock().unwrap().clear();
                            event_list_state.select(None);
                        }
                        _ => {}
                    },
                }
            } else if let CEvent::Paste(text) = &evt {
                match &mut state {
                    AppState::EnterCommandPayload { textarea, .. }
                    | AppState::EnterQueryParams { textarea, .. } => {
                        let parts: Vec<&str> = text.split('\n').collect();
                        for (idx, part) in parts.iter().enumerate() {
                            let clean = part.strip_suffix('\r').unwrap_or(part);
                            textarea.insert_str(clean);
                            if idx + 1 < parts.len() {
                                textarea.insert_newline();
                            }
                        }
                    }
                    AppState::EnterEventFilter { textarea, .. } => {
                        let single_line = text
                            .lines()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ");
                        textarea.insert_str(&single_line);
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
    use ratatui::backend::TestBackend;
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

    #[test]
    fn wrap_text_breaks_long_descriptions_and_unbroken_tokens() {
        let wrapped = wrap_text(
            "Opens a new bank account with an owner name and initial balance.",
            24,
        );
        assert!(wrapped.len() >= 3);
        assert!(wrapped.iter().all(|line| line.chars().count() <= 24));

        let long_token = wrap_text(
            "/bank-account-example/finance/account/00000000-0000-0000-0000-000000000001",
            20,
        );
        assert!(long_token.len() >= 4);
        assert!(long_token.iter().all(|line| line.chars().count() <= 20));
    }

    #[test]
    fn event_entry_filter_is_case_insensitive_across_fields() {
        let entry = EventEntry {
            subject: "/bank-account-example/finance/account/acc-1".to_owned(),
            event_type: "account-opened".to_owned(),
            payload_pretty: "{\n  \"owner-name\": \"Alice\"\n}".to_owned(),
            payload_compact: "{\"owner-name\":\"Alice\"}".to_owned(),
        };

        assert!(entry.matches_filter(""));
        assert!(entry.matches_filter("account-opened"));
        assert!(entry.matches_filter(&"ALICE".to_lowercase()));
        assert!(entry.matches_filter(&"ACC-1".to_lowercase()));
        assert!(!entry.matches_filter("bob"));
    }

    #[test]
    fn render_ui_distinguishes_success_and_error_results_and_wraps_events() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut log = VecDeque::new();
        log.push_front(EventEntry {
            subject: "/bank-account-example/finance/account/00000000-0000-0000-0000-000000000001"
                .to_owned(),
            event_type: "account-opened".to_owned(),
            payload_pretty: "{\n  \"owner-name\": \"Alice\",\n  \"balance\": 100\n}".to_owned(),
            payload_compact: "{\"owner-name\":\"Alice\",\"balance\":100}".to_owned(),
        });
        let mut event_ls = ListState::default();

        let mut error_state = AppState::ResultView {
            kind: ResultKind::Error,
            title: "Command Failed: withdraw-funds".to_owned(),
            body: "insufficient-funds: Withdrawal amount cannot exceed current balance.".to_owned(),
            scroll: 0,
        };

        terminal
            .draw(|f| {
                render_ui(
                    f,
                    "bank-account-example",
                    "customer",
                    "alice",
                    &log,
                    &mut event_ls,
                    &mut error_state,
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        assert_eq!(buf[(0, 0)].fg, Color::Red);
        let screen_text: String = (0..24)
            .map(|y| (0..100).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(screen_text.contains("[ERROR] Command Failed: withdraw-funds"));
        assert!(screen_text.contains("Withdrawal amount"));
        assert!(screen_text.contains("filter: \"alice\""));
        assert!(screen_text.contains("balance"));

        let mut success_state = AppState::ResultView {
            kind: ResultKind::Success,
            title: "Command: open-account".to_owned(),
            body: "Command 'open-account' succeeded — 1 event(s) emitted.".to_owned(),
            scroll: 0,
        };
        terminal
            .draw(|f| {
                render_ui(
                    f,
                    "bank-account-example",
                    "customer",
                    "",
                    &log,
                    &mut event_ls,
                    &mut success_state,
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        assert_eq!(buf[(0, 0)].fg, Color::Green);
    }
}
