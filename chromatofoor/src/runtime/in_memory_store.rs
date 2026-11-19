//! In-memory [`EventStore`](architecturekit::EventStore) implementation for
//! local development, embedded runtimes, and tests.

use std::{
    collections::{BTreeSet, HashMap},
    future,
    sync::{Arc, Mutex, PoisonError},
};

use architecturekit::{
    EventStore, EventStoreError, EventStoreFuture, EventStream, EventTypeStream, Revision, Subject,
    eventsourcingdb::{
        self, EventCandidate,
        error::{ClientError, EventError},
        request_options::{
            Bound, BoundType, EventType, ObserveEventMissingStrategy, ObserveEventsOptions,
            Ordering, ReadEventMissingStrategy, ReadEventsOptions,
        },
    },
};
use chrono::Utc;
use futures_util::stream;
use serde_json::{Value, json};
use tokio::sync::watch;

/// An in-memory [`EventStore`] that keeps events and schemas in memory and
/// wakes live projections as soon as new events are written.
///
/// Cloning an [`InMemoryEventStore`] shares the same underlying events and
/// schemas, so several [`Store`](architecturekit::Store) instances can read
/// and write the same in-memory event store concurrently.
///
/// ```
/// use chromatofoor::{InMemoryEventStore, architecturekit::Store};
///
/// let store = Store::builder(InMemoryEventStore::new(), "https://library.local").build();
/// ```
#[derive(Clone, Debug)]
pub struct InMemoryEventStore {
    inner: Arc<Mutex<Inner>>,
    written: Arc<watch::Sender<usize>>,
}

/// The synchronized state of an [`InMemoryEventStore`].
#[derive(Debug, Default)]
struct Inner {
    events: Vec<eventsourcingdb::Event>,
    schemas: HashMap<String, Value>,
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryEventStore {
    /// Creates an empty in-memory event store.
    #[must_use]
    pub fn new() -> Self {
        Self::from_events(Vec::new())
    }

    /// Creates an in-memory event store seeded with existing stored `events`.
    #[must_use]
    pub fn from_events(events: Vec<eventsourcingdb::Event>) -> Self {
        let (written, _) = watch::channel(events.len());
        Self {
            inner: Arc::new(Mutex::new(Inner {
                events,
                schemas: HashMap::new(),
            })),
            written: Arc::new(written),
        }
    }

    /// Resolves the starting event index and optional wait target for
    /// [`EventStore::observe_events`].
    fn resolve_observe_start(
        &self,
        options: &ObserveEventsOptions<'_>,
    ) -> Result<(usize, Option<(String, String)>), EventStoreError> {
        match (&options.from_latest_event, &options.lower_bound) {
            (Some(_), Some(_)) => Err(EventStoreError::permanent(
                "from_latest_event and lower_bound cannot be combined",
            )),
            (Some(latest), None) => {
                if latest.subject.parse::<Subject>().is_err() {
                    return Err(EventStoreError::permanent(format!(
                        "malformed subject {:?}",
                        latest.subject
                    )));
                }
                let wait_target = match latest.if_event_is_missing {
                    ObserveEventMissingStrategy::ReadEverything => None,
                    ObserveEventMissingStrategy::WaitForEvent => {
                        Some((latest.subject.to_owned(), latest.ty.to_owned()))
                    }
                };
                let found = self
                    .inner
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .events
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, event)| event.subject() == latest.subject && event.ty() == latest.ty)
                    .map(|(index, _)| index);
                Ok(match (found, wait_target) {
                    (Some(index), _) => (index, None),
                    (None, None) => (0, None),
                    (None, Some(target)) => (0, Some(target)),
                })
            }
            (None, Some(bound)) => {
                let start = usize::try_from(bound_start(bound)?).unwrap_or(usize::MAX);
                Ok((start, None))
            }
            (None, None) => Ok((0, None)),
        }
    }
}

/// Finds the next matching event in `inner` for an active observation stream.
fn next_observed_event(
    inner: &Mutex<Inner>,
    subject: &str,
    recursive: bool,
    next_index: &mut usize,
    wait_for: &mut Option<(String, String)>,
) -> Option<eventsourcingdb::Event> {
    let guard = inner.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((target_subject, target_ty)) = wait_for
        && let Some((idx, _)) = guard
            .events
            .iter()
            .enumerate()
            .rev()
            .find(|(_, event)| event.subject() == target_subject && event.ty() == target_ty)
    {
        *next_index = idx;
        *wait_for = None;
    }

    if wait_for.is_some() {
        return None;
    }

    while *next_index < guard.events.len() {
        let event = &guard.events[*next_index];
        *next_index += 1;
        if subject_matches(event.subject(), subject, recursive) {
            return Some(event.clone());
        }
    }

    None
}

impl EventStore for InMemoryEventStore {
    fn read_events<'a, 'fut>(
        &'a self,
        subject: &'fut str,
        options: Option<ReadEventsOptions<'fut>>,
    ) -> EventStoreFuture<'fut, EventStream<'a>>
    where
        'a: 'fut,
    {
        Box::pin(async move {
            if subject.parse::<Subject>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed subject {subject:?}"
                )));
            }

            let options = options.unwrap_or_default();
            let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);

            let start = match (&options.from_latest_event, &options.lower_bound) {
                (Some(_), Some(_)) => {
                    return Err(EventStoreError::permanent(
                        "from_latest_event and lower_bound cannot be combined",
                    ));
                }
                (Some(latest), None) => {
                    if latest.subject.parse::<Subject>().is_err() {
                        return Err(EventStoreError::permanent(format!(
                            "malformed subject {:?}",
                            latest.subject
                        )));
                    }
                    let found = guard
                        .events
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, event)| {
                            event.subject() == latest.subject && event.ty() == latest.ty
                        })
                        .map(|(index, _)| index as u64);

                    match (found, &latest.if_event_is_missing) {
                        (Some(index), _) => index,
                        (None, ReadEventMissingStrategy::ReadEverything) => 0,
                        (None, ReadEventMissingStrategy::ReadNothing) => {
                            let empty: EventStream<'a> = Box::pin(stream::empty());
                            return Ok(empty);
                        }
                    }
                }
                (None, Some(bound)) => bound_start(bound)?,
                (None, None) => 0,
            };

            let end = match &options.upper_bound {
                Some(bound) => bound_end(bound)?,
                None => u64::MAX,
            };

            if start >= end {
                return Err(EventStoreError::permanent(
                    "range is unsatisfiable: lower bound is not below upper bound",
                ));
            }

            let mut matched: Vec<eventsourcingdb::Event> = guard
                .events
                .iter()
                .enumerate()
                .filter(|(index, event)| {
                    let id = *index as u64;
                    id >= start
                        && id < end
                        && subject_matches(event.subject(), subject, options.recursive)
                })
                .map(|(_, event)| event.clone())
                .collect();
            drop(guard);

            if matches!(options.order, Some(Ordering::Antichronological)) {
                matched.reverse();
            }

            let stream: EventStream<'a> = Box::pin(stream::iter(matched.into_iter().map(Ok)));
            Ok(stream)
        })
    }

    fn observe_events<'a, 'fut>(
        &'a self,
        subject: &'fut str,
        options: Option<ObserveEventsOptions<'fut>>,
    ) -> EventStoreFuture<'fut, EventStream<'a>>
    where
        'a: 'fut,
    {
        Box::pin(async move {
            if subject.parse::<Subject>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed subject {subject:?}"
                )));
            }

            let options = options.unwrap_or_default();
            let recursive = options.recursive;
            let subject_owned = subject.to_owned();
            let (next_index, wait_for) = self.resolve_observe_start(&options)?;

            let inner = Arc::clone(&self.inner);
            let rx = self.written.subscribe();
            let stream: EventStream<'a> = Box::pin(stream::unfold(
                (inner, rx, subject_owned, recursive, next_index, wait_for),
                |(inner, mut rx, subject, recursive, mut next_index, mut wait_for)| async move {
                    loop {
                        let found = next_observed_event(
                            &inner,
                            &subject,
                            recursive,
                            &mut next_index,
                            &mut wait_for,
                        );
                        if let Some(event) = found {
                            return Some((
                                Ok(event),
                                (inner, rx, subject, recursive, next_index, wait_for),
                            ));
                        }

                        if rx.changed().await.is_err() {
                            future::pending::<()>().await;
                        }
                    }
                },
            ));

            Ok(stream)
        })
    }

    fn write_events(
        &self,
        candidates: Vec<EventCandidate>,
        preconditions: Vec<eventsourcingdb::Precondition>,
    ) -> EventStoreFuture<'_, Vec<eventsourcingdb::Event>> {
        Box::pin(async move {
            if candidates.is_empty() {
                return Err(EventStoreError::permanent("events must not be empty"));
            }

            for candidate in &candidates {
                if candidate.ty.is_empty() {
                    return Err(EventStoreError::from(ClientError::InvalidEventType));
                }
                if candidate.subject.parse::<Subject>().is_err() {
                    return Err(EventStoreError::permanent(format!(
                        "malformed subject {:?}",
                        candidate.subject
                    )));
                }
                if candidate.source.is_empty() {
                    return Err(EventStoreError::permanent("source must not be empty"));
                }
            }

            let (written, total_events) = {
                let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);

                for (index, precondition) in preconditions.iter().enumerate() {
                    if !evaluate_precondition(&guard.events, precondition)? {
                        return Err(EventStoreError::PreconditionViolated {
                            index: Some(index),
                            description: format!(
                                "/preconditions/{index}: precondition does not hold"
                            ),
                        });
                    }
                }

                let mut predecessor_hash = guard
                    .events
                    .last()
                    .map_or_else(|| "0".repeat(64), |event| event.hash().to_owned());
                let mut staged = Vec::with_capacity(candidates.len());

                for (next_id, candidate) in (guard.events.len() as u64..).zip(candidates.iter()) {
                    let event = build_stored_event(next_id, &predecessor_hash, candidate)?;
                    event.hash().clone_into(&mut predecessor_hash);
                    staged.push(event);
                }

                guard.events.extend(staged.iter().cloned());
                (staged, guard.events.len())
            };

            let _ = self.written.send_replace(total_events);
            Ok(written)
        })
    }

    fn read_event_types(&self) -> EventStoreFuture<'_, EventTypeStream<'_>> {
        Box::pin(async move {
            let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            let mut names: BTreeSet<String> = guard.schemas.keys().cloned().collect();
            for event in &guard.events {
                let _ = names.insert(event.ty().to_owned());
            }

            let types: Vec<EventType> = names
                .into_iter()
                .map(|ty| {
                    let has_events = guard.events.iter().any(|event| event.ty() == ty);
                    let schema = guard.schemas.get(&ty).cloned();
                    EventType {
                        ty,
                        has_events,
                        schema,
                    }
                })
                .collect();
            drop(guard);

            let stream: EventTypeStream<'_> = Box::pin(stream::iter(types.into_iter().map(Ok)));
            Ok(stream)
        })
    }

    fn register_event_schema<'a>(
        &'a self,
        event_type: &'a str,
        schema: &'a Value,
    ) -> EventStoreFuture<'a, ()> {
        Box::pin(async move {
            if event_type.is_empty() {
                return Err(EventStoreError::from(ClientError::InvalidEventType));
            }
            if !schema.is_object() {
                return Err(EventStoreError::from(ClientError::JsonSchemaError));
            }

            let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if guard.schemas.contains_key(event_type) {
                return Err(EventStoreError::SchemaConflict {
                    description: format!("schema for {event_type:?} is already registered"),
                });
            }

            let _ = guard.schemas.insert(event_type.to_owned(), schema.clone());
            Ok(())
        })
    }
}

/// Tells whether `event_subject` matches `target_subject`, either on the
/// exact subject alone or recursively across all subjects below it.
fn subject_matches(event_subject: &str, target_subject: &str, recursive: bool) -> bool {
    if event_subject == target_subject {
        return true;
    }
    if !recursive {
        return false;
    }
    if target_subject == "/" {
        return true;
    }

    event_subject
        .strip_prefix(target_subject)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Parses the inclusive start index of a lower bound.
fn bound_start(bound: &Bound<'_>) -> Result<u64, EventStoreError> {
    let revision: Revision = bound
        .id
        .parse()
        .map_err(|_| EventStoreError::permanent(format!("malformed event ID {:?}", bound.id)))?;

    Ok(match bound.bound_type {
        BoundType::Inclusive => revision.get(),
        BoundType::Exclusive => revision.get() + 1,
    })
}

/// Parses the exclusive end index of an upper bound.
fn bound_end(bound: &Bound<'_>) -> Result<u64, EventStoreError> {
    let revision: Revision = bound
        .id
        .parse()
        .map_err(|_| EventStoreError::permanent(format!("malformed event ID {:?}", bound.id)))?;

    Ok(match bound.bound_type {
        BoundType::Inclusive => revision.get() + 1,
        BoundType::Exclusive => revision.get(),
    })
}

/// Evaluates a single [`eventsourcingdb::Precondition`] against `events`.
fn evaluate_precondition(
    events: &[eventsourcingdb::Event],
    precondition: &eventsourcingdb::Precondition,
) -> Result<bool, EventStoreError> {
    match precondition {
        eventsourcingdb::Precondition::IsSubjectPristine {
            subject, options, ..
        } => {
            if subject.parse::<Subject>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed subject {subject:?}"
                )));
            }
            let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
            Ok(!events
                .iter()
                .any(|event| subject_matches(event.subject(), subject, recursive)))
        }
        eventsourcingdb::Precondition::IsSubjectPopulated {
            subject, options, ..
        } => {
            if subject.parse::<Subject>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed subject {subject:?}"
                )));
            }
            let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
            Ok(events
                .iter()
                .any(|event| subject_matches(event.subject(), subject, recursive)))
        }
        eventsourcingdb::Precondition::IsSubjectOnEventId {
            subject,
            event_id,
            options,
            ..
        } => {
            if subject.parse::<Subject>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed subject {subject:?}"
                )));
            }
            if event_id.parse::<Revision>().is_err() {
                return Err(EventStoreError::permanent(format!(
                    "malformed event ID {event_id:?}"
                )));
            }
            let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
            let latest = events
                .iter()
                .rev()
                .find(|event| subject_matches(event.subject(), subject, recursive));
            Ok(latest.is_some_and(|event| event.id() == event_id))
        }
        eventsourcingdb::Precondition::IsEventQLQueryTrue { query, .. } => {
            evaluate_simple_eventql(events, query)
        }
    }
}

/// Evaluates basic boolean EventQL queries of the form
/// `FROM e IN events [WHERE ...] PROJECT INTO ...` for in-memory testing.
fn evaluate_simple_eventql(
    events: &[eventsourcingdb::Event],
    query: &str,
) -> Result<bool, EventStoreError> {
    let trimmed = query.trim();
    let Some(rest) = trimmed.strip_prefix("FROM e IN events") else {
        return Err(EventStoreError::permanent(format!(
            "unsupported EventQL query {query:?}"
        )));
    };
    let rest = rest.trim();

    let (where_clause, projection) = if let Some(after_where) = rest.strip_prefix("WHERE ") {
        let Some((cond, proj)) = after_where.split_once("PROJECT INTO") else {
            return Err(EventStoreError::permanent(format!(
                "unsupported EventQL query {query:?}"
            )));
        };
        (Some(cond.trim()), proj.trim())
    } else if let Some(proj) = rest.strip_prefix("PROJECT INTO") {
        (None, proj.trim())
    } else {
        return Err(EventStoreError::permanent(format!(
            "unsupported EventQL query {query:?}"
        )));
    };

    let mut count = 0_usize;
    for event in events {
        let matches = match where_clause {
            None => true,
            Some(clause) => matches_where_clause(event, clause)?,
        };
        if matches {
            count += 1;
        }
    }

    match projection {
        "true" => Ok(true),
        "false" => Ok(false),
        _ if projection.starts_with("COUNT()") => {
            let rhs = projection.trim_start_matches("COUNT()").trim();
            if let Some(val) = rhs.strip_prefix("==") {
                let target: usize = val.trim().parse().map_err(|_| {
                    EventStoreError::permanent(format!("unsupported EventQL query {query:?}"))
                })?;
                Ok(count == target)
            } else if let Some(val) = rhs.strip_prefix('<') {
                let target: usize = val.trim().parse().map_err(|_| {
                    EventStoreError::permanent(format!("unsupported EventQL query {query:?}"))
                })?;
                Ok(count < target)
            } else if let Some(val) = rhs.strip_prefix('>') {
                let target: usize = val.trim().parse().map_err(|_| {
                    EventStoreError::permanent(format!("unsupported EventQL query {query:?}"))
                })?;
                Ok(count > target)
            } else {
                Err(EventStoreError::permanent(format!(
                    "unsupported EventQL query {query:?}"
                )))
            }
        }
        _ => Err(EventStoreError::permanent(format!(
            "unsupported EventQL query {query:?}"
        ))),
    }
}

/// Evaluates a simple `e.subject == '...'` or `e.type == '...'` filter.
fn matches_where_clause(
    event: &eventsourcingdb::Event,
    clause: &str,
) -> Result<bool, EventStoreError> {
    if let Some(rhs) = clause.strip_prefix("e.subject ==") {
        let literal = unquote_single(rhs.trim()).ok_or_else(|| {
            EventStoreError::permanent(format!("unsupported WHERE clause {clause:?}"))
        })?;
        Ok(event.subject() == literal)
    } else if let Some(rhs) = clause.strip_prefix("e.type ==") {
        let literal = unquote_single(rhs.trim()).ok_or_else(|| {
            EventStoreError::permanent(format!("unsupported WHERE clause {clause:?}"))
        })?;
        Ok(event.ty() == literal)
    } else {
        Err(EventStoreError::permanent(format!(
            "unsupported WHERE clause {clause:?}"
        )))
    }
}

/// Strips surrounding single quotes from an EventQL string literal.
fn unquote_single(input: &str) -> Option<&str> {
    input.strip_prefix('\'')?.strip_suffix('\'')
}

/// Builds a stored [`eventsourcingdb::Event`] with a valid v2 SHA-256 hash.
fn build_stored_event(
    id: u64,
    predecessor_hash: &str,
    candidate: &EventCandidate,
) -> Result<eventsourcingdb::Event, EventStoreError> {
    let time = Utc::now();
    let mut payload = json!({
        "specversion": "1.0",
        "id": id.to_string(),
        "time": time,
        "source": &candidate.source,
        "subject": &candidate.subject,
        "type": &candidate.ty,
        "datacontenttype": "application/json",
        "data": &candidate.data,
        "hash": "esdb:hash:v2:",
        "predecessorhash": predecessor_hash,
    });
    if let Some(traceinfo) = &candidate.traceinfo {
        payload["traceparent"] = Value::String(traceinfo.traceparent().to_owned());
        if let Some(tracestate) = traceinfo.tracestate() {
            payload["tracestate"] = Value::String(tracestate.to_owned());
        }
    }

    let draft: eventsourcingdb::Event =
        serde_json::from_value(payload.clone()).map_err(ClientError::from)?;
    let hash = match draft.verify_hash() {
        Ok(()) => draft.hash().to_owned(),
        Err(EventError::HashVerificationFailed { actual, .. }) => actual,
        Err(error) => return Err(EventStoreError::Permanent(Box::new(error))),
    };

    payload["hash"] = Value::String(hash);
    let event: eventsourcingdb::Event =
        serde_json::from_value(payload).map_err(ClientError::from)?;

    Ok(event)
}
