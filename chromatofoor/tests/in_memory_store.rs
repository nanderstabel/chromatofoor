//! Integration tests for [`chromatofoor::InMemoryEventStore`] exercising
//! [`architecturekit::Store`] operations (`load`, `execute`, `conflict_retries`,
//! `write`, `read`, live projections, views, and `register_schemas`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use architecturekit::{
    CancellationToken, Command, Decider, DomainError, Envelope, Event, EventOn, EventSchema,
    Events, ExecuteError, IfLatestEventIsMissing, InMemoryView, KeyedView, LowerBound, Order,
    Precondition, Projection, ProjectionOptions, ReadOptions, RegisterSchemasError, Revision,
    State, Store, Subject, Subjects, Transient, UpperBound, ViewError, eventsourcingdb,
    revision_of, wait_caught_up, wait_for_written,
};
use chromatofoor::InMemoryEventStore;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

const SOURCE: &str = "https://chromatofoor.test";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.incremented")]
struct Incremented {
    by: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.reset")]
struct Reset {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.annotated")]
struct Annotated {
    note: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Events)]
enum CounterEvent {
    Incremented(Incremented),
    Reset(Reset),
    Annotated(Annotated),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Counter {
    total: i64,
}

impl State for Counter {
    type Event = CounterEvent;

    fn evolve(self, event: &CounterEvent) -> Self {
        match event {
            CounterEvent::Incremented(inc) => Self {
                total: self.total + inc.by,
            },
            CounterEvent::Reset(_) => Self { total: 0 },
            CounterEvent::Annotated(_) => self,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CounterFromLatest(Counter);

impl State for CounterFromLatest {
    type Event = CounterEvent;

    fn from_latest() -> Option<&'static str> {
        Some(Reset::EVENT_TYPE)
    }

    fn evolve(self, event: &CounterEvent) -> Self {
        Self(self.0.evolve(event))
    }
}

#[derive(Clone)]
struct Increment {
    subject: Subject,
    by: i64,
    limit: Option<i64>,
    declare: Option<Arc<dyn Fn() -> Vec<Precondition> + Send + Sync>>,
}

impl Increment {
    fn new(subject: &Subject, by: i64) -> Self {
        Self {
            subject: subject.clone(),
            by,
            limit: None,
            declare: None,
        }
    }

    fn on_state_read(self) -> Self {
        self.declaring(|| vec![Precondition::on_state_read()])
    }

    fn declaring(mut self, f: impl Fn() -> Vec<Precondition> + Send + Sync + 'static) -> Self {
        self.declare = Some(Arc::new(f));
        self
    }
}

impl Command for Increment {
    fn subject(&self) -> Subject {
        self.subject.clone()
    }

    fn preconditions(&self) -> Vec<Precondition> {
        match &self.declare {
            Some(declare) => declare(),
            None => vec![Precondition::unconditionally()],
        }
    }
}

fn counter_decider() -> Decider<Increment, Counter, DomainError> {
    Decider::new(|command: Increment, _state: Counter| async move {
        Ok(vec![CounterEvent::Incremented(Incremented {
            by: command.by,
        })])
    })
}

async fn total_in(store: &Store, subject: &Subject) -> i64 {
    store
        .load::<Counter>(subject)
        .await
        .expect("loads counter")
        .total
}

#[derive(Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.registered")]
struct Registered {
    id: String,
}

#[derive(Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.titled")]
struct Titled {
    title: String,
}

#[derive(Serialize, Deserialize, Event)]
#[event(type = "io.chromatofoor.test.titled")]
struct TitledOtherwise {
    name: String,
}

fn memory_store() -> (Store, InMemoryEventStore) {
    let event_store = InMemoryEventStore::new();
    let store = Store::builder(event_store.clone(), SOURCE).build();
    (store, event_store)
}

#[tokio::test]
async fn loads_and_writes_events_with_hash_verification_in_memory() {
    let (store, _) = memory_store();
    let subject = Subject::new("/counters/1");

    let initial: Counter = store.load(&subject).await.expect("pristine subject loads");
    assert_eq!(initial, Counter::default());

    let written = store
        .write(
            vec![
                EventOn::new(subject.clone(), &Incremented { by: 3 }),
                EventOn::new(subject.clone(), &Incremented { by: 4 }),
            ],
            vec![Precondition::on_pristine_subject(subject.clone())],
        )
        .await
        .expect("events are written");

    assert_eq!(written.len(), 2);
    assert!(written[0].verify_hash().is_ok());
    assert!(written[1].verify_hash().is_ok());
    assert_eq!(written[1].predecessorhash(), written[0].hash());

    let loaded: Counter = store.load(&subject).await.expect("populated subject loads");
    assert_eq!(loaded.total, 7);
}

#[tokio::test]
async fn executes_commands_and_retries_conflicts_on_state_read_in_memory() {
    let event_store = InMemoryEventStore::new();
    let store = Store::builder(event_store.clone(), SOURCE)
        .conflict_retries(2)
        .build();
    let interfering_store = Arc::new(Store::builder(Arc::new(event_store), SOURCE).build());
    let subject = Subject::new("/counters/retry");
    let decisions = Arc::new(AtomicUsize::new(0));

    let counted = Arc::clone(&decisions);
    let decider = Decider::<Increment, Counter, DomainError>::new(move |command, state| {
        let counted = Arc::clone(&counted);
        let interfering_store = Arc::clone(&interfering_store);
        async move {
            let attempt = counted.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                let _ = interfering_store
                    .write(
                        vec![EventOn::new(
                            command.subject.clone(),
                            &Incremented { by: 100 },
                        )],
                        vec![Precondition::unconditionally()],
                    )
                    .await
                    .expect("interfering write succeeds");
            }
            if let Some(limit) = command.limit
                && state.total + command.by > limit
            {
                return Err(DomainError::new("limit exceeded"));
            }
            Ok(vec![CounterEvent::Incremented(Incremented {
                by: command.by,
            })])
        }
    });

    let written = store
        .execute(&decider, Increment::new(&subject, 1).on_state_read())
        .await
        .expect("second attempt succeeds");

    assert_eq!(written.len(), 1);
    assert_eq!(decisions.load(Ordering::SeqCst), 2);
    assert_eq!(total_in(&store, &subject).await, 101);
}

#[tokio::test]
async fn aborts_conflict_retries_immediately_when_non_state_read_precondition_fails_in_memory() {
    let (store, event_store) = memory_store();
    let retrying = Store::builder(event_store, SOURCE)
        .conflict_retries(2)
        .build();
    let subject = Subject::new("/counters/outdated");

    let first = store
        .execute(&counter_decider(), Increment::new(&subject, 1))
        .await
        .expect("first write");
    let _ = store
        .execute(&counter_decider(), Increment::new(&subject, 1))
        .await
        .expect("second write");

    let decisions = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&decisions);
    let decider = Decider::<Increment, Counter, DomainError>::new(move |command, _| {
        let _ = counted.fetch_add(1, Ordering::SeqCst);
        async move {
            Ok(vec![CounterEvent::Incremented(Incremented {
                by: command.by,
            })])
        }
    });

    let on = subject.clone();
    let outdated = revision_of(&first).expect("valid revision");
    let failure = retrying
        .execute(
            &decider,
            Increment::new(&subject, 1).declaring(move || {
                vec![
                    Precondition::on_state_read(),
                    Precondition::on_event_id(on.clone(), outdated),
                ]
            }),
        )
        .await
        .expect_err("caller revision is outdated");

    assert!(
        matches!(failure, ExecuteError::Transient(Transient::Conflict(_))),
        "{failure:?}"
    );
    assert_eq!(
        decisions.load(Ordering::SeqCst),
        1,
        "does not retry when the violated precondition is not on_state_read"
    );
}

#[tokio::test]
async fn evaluates_eventql_and_recursive_preconditions_in_memory() {
    let (store, _) = memory_store();
    let subject = Subject::new("/books/42");
    let child = Subject::new("/books/42/copies/1");

    let _ = store
        .write(
            vec![EventOn::new(child.clone(), &Incremented { by: 1 })],
            vec![Precondition::require(
                eventsourcingdb::Precondition::is_eventql_query_true(
                    "FROM e IN events WHERE e.subject == '/books/42/copies/1' PROJECT INTO COUNT() == 0",
                ),
            )],
        )
        .await
        .expect("query holds when pristine");

    let conflict = store
        .write(
            vec![EventOn::new(subject.clone(), &Incremented { by: 1 })],
            vec![Precondition::require(
                eventsourcingdb::Precondition::is_subject_pristine(
                    "/books/42",
                    Some(eventsourcingdb::PreconditionOptions { recursive: true }),
                ),
            )],
        )
        .await
        .expect_err("child subject has an event");

    assert!(matches!(
        conflict,
        architecturekit::WriteError::Transient(Transient::Conflict(_))
    ));
}

#[tokio::test]
async fn reads_with_bounds_ordering_and_from_latest_event_in_memory() {
    let (store, _) = memory_store();
    let subject = Subject::new("/counters/read");

    let _ = store
        .write(
            vec![
                EventOn::new(subject.clone(), &Incremented { by: 1 }),
                EventOn::new(subject.clone(), &Reset {}),
                EventOn::new(subject.clone(), &Incremented { by: 5 }),
                EventOn::new(
                    subject.clone(),
                    &Annotated {
                        note: "done".to_owned(),
                    },
                ),
            ],
            vec![Precondition::unconditionally()],
        )
        .await
        .expect("events written");

    let from_latest: CounterFromLatest =
        store.load(&subject).await.expect("loads from latest reset");
    assert_eq!(from_latest.0.total, 5);

    let reverse_options = ReadOptions {
        lower_bound: LowerBound::FromEvent(Revision::from(1)),
        upper_bound: UpperBound::BeforeEvent(Revision::from(3)),
        order: Order::NewestFirst,
    };
    let events: Vec<_> = store
        .read(&Subjects::exact(subject.clone()), &reverse_options)
        .await
        .expect("read succeeds")
        .try_collect()
        .await
        .expect("stream succeeds");
    let ids: Vec<_> = events.iter().map(eventsourcingdb::Event::id).collect();
    assert_eq!(ids, ["2", "1"]);

    let missing_options = ReadOptions {
        lower_bound: LowerBound::FromLatestEvent {
            subject: subject.clone(),
            event_type: "io.chromatofoor.test.missing".to_owned(),
            if_missing: IfLatestEventIsMissing::ReadNothing,
        },
        ..ReadOptions::default()
    };
    let empty: Vec<_> = store
        .read(&Subjects::exact(subject), &missing_options)
        .await
        .expect("read succeeds")
        .try_collect()
        .await
        .expect("stream succeeds");
    assert_eq!(empty.len(), 0);
}

struct TotalsProjection {
    totals: InMemoryView<String, i64>,
}

impl Projection for TotalsProjection {
    type Event = CounterEvent;
    type Error = ViewError;

    async fn apply(&self, event: Envelope<CounterEvent>) -> Result<(), ViewError> {
        let key = event.subject.as_str();
        match &event.data {
            CounterEvent::Incremented(inc) => {
                let _ = self.totals.upsert(key, &event, |total| *total += inc.by)?;
            }
            CounterEvent::Reset(_) => {
                let _ = self.totals.upsert(key, &event, |total| *total = 0)?;
            }
            CounterEvent::Annotated(_) => {}
        }
        Ok(())
    }

    fn seen(&self, event_id: Revision) {
        self.totals.seen(event_id);
    }
}

#[tokio::test]
async fn drives_live_projections_and_wakes_wait_for_written_in_memory() {
    let (store, _) = memory_store();
    let subject = Subject::new("/counters/live");

    let _ = store
        .execute(&counter_decider(), Increment::new(&subject, 10))
        .await
        .expect("initial event");

    let projection = Arc::new(TotalsProjection {
        totals: InMemoryView::new(|total: &i64| {
            let _ = total;
            "/counters/live".to_owned()
        }),
    });
    let token = CancellationToken::new();
    let run = store.start_projection(
        Subjects::tree(Subject::new("/counters")),
        Arc::clone(&projection),
        token.clone(),
        ProjectionOptions::default(),
    );

    wait_caught_up(&[&run]).await.expect("catches up");

    let written = store
        .execute(&counter_decider(), Increment::new(&subject, 5))
        .await
        .expect("live event");

    wait_for_written(&projection.totals, &written, Duration::from_secs(1))
        .await
        .expect("projection applies live event");

    let Ok(total) = projection.totals.get("/counters/live").await;
    assert_eq!(total, Some(15));
    token.cancel();
}

#[tokio::test]
async fn registers_and_verifies_schemas_in_memory() {
    let (store, _) = memory_store();

    store
        .register_schemas([EventSchema::of::<Registered>(), EventSchema::of::<Titled>()])
        .await
        .expect("schemas registered");

    store
        .register_schemas([EventSchema::of::<Registered>(), EventSchema::of::<Titled>()])
        .await
        .expect("re-registering identical schemas succeeds");

    let conflict = store
        .register_schemas([EventSchema::of::<TitledOtherwise>()])
        .await
        .expect_err("conflicting schema for same event type fails");

    assert!(matches!(conflict, RegisterSchemasError::Permanent(_)));
}
