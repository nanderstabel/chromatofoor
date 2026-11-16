//! Timer definitions, deadline arithmetic, and background timer scheduling
//! (`setTimers` / `cancelTimers`) for [`ProcessManager`].

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use tokio::task::JoinHandle;

use crate::integration::{
    ProcessInstanceEvent, ProcessInstanceState, ProcessManager, ProcessTimerFired,
};

/// Calendar/wall-clock unit for a relative process-manager timer (`timers[].after.unit` in ESDM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerUnit {
    /// Seconds (`seconds`).
    Seconds,
    /// Minutes (`minutes`).
    Minutes,
    /// Hours (`hours`).
    Hours,
    /// Calendar days (`days`).
    Days,
    /// Calendar weeks (`weeks`).
    Weeks,
    /// Calendar months (`months`).
    Months,
    /// Calendar years (`years`).
    Years,
}

impl TimerUnit {
    /// Adds `value` units of `self` to `base` using calendar/wall-clock arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use chromatofoor::{
    ///     chrono::{TimeZone, Utc},
    ///     integration::TimerUnit,
    /// };
    ///
    /// let base = Utc.with_ymd_and_hms(2026, 1, 31, 12, 0, 0).unwrap();
    /// assert_eq!(
    ///     TimerUnit::Minutes.add_to(base, 15),
    ///     Utc.with_ymd_and_hms(2026, 1, 31, 12, 15, 0).unwrap()
    /// );
    /// assert_eq!(
    ///     TimerUnit::Months.add_to(base, 6),
    ///     Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0).unwrap()
    /// );
    /// assert_eq!(
    ///     TimerUnit::Years.add_to(base, u64::MAX),
    ///     chromatofoor::chrono::DateTime::<Utc>::MAX_UTC
    /// );
    /// ```
    #[must_use]
    pub fn add_to(
        self,
        base: chrono::DateTime<chrono::Utc>,
        value: u64,
    ) -> chrono::DateTime<chrono::Utc> {
        let signed = i64::try_from(value).unwrap_or(i64::MAX);
        match self {
            Self::Seconds => chrono::TimeDelta::try_seconds(signed)
                .and_then(|delta| base.checked_add_signed(delta))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Minutes => chrono::TimeDelta::try_minutes(signed)
                .and_then(|delta| base.checked_add_signed(delta))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Hours => chrono::TimeDelta::try_hours(signed)
                .and_then(|delta| base.checked_add_signed(delta))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Days => base
                .checked_add_days(chrono::Days::new(value))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Weeks => value
                .checked_mul(7)
                .and_then(|days| base.checked_add_days(chrono::Days::new(days)))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Months => u32::try_from(value)
                .ok()
                .and_then(|months| base.checked_add_months(chrono::Months::new(months)))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
            Self::Years => u32::try_from(value)
                .ok()
                .and_then(|years| years.checked_mul(12))
                .and_then(|months| base.checked_add_months(chrono::Months::new(months)))
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC),
        }
    }
}

/// Schedule specification for a [`ProcessTimer`] (`timers[].after` or `timers[].at` in ESDM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerSchedule {
    /// Relative delay from when the timer is armed (`after: { value, unit }`).
    After {
        /// Positive duration magnitude (`>= 1`).
        value: u64,
        /// Duration unit.
        unit: TimerUnit,
    },
    /// Absolute timestamp drawn from a field on the process-manager instance's state (`at: <field>`).
    At {
        /// Name of the state field holding the target timestamp.
        field: &'static str,
    },
}

/// Extracts a target UTC firing timestamp from a [`ProcessManager`] state field
/// referenced by `timers = [MyTimer(at = <field>)]`.
pub trait AsTimerTimestamp {
    /// Returns the target UTC timestamp, or `None` if unset or unparseable.
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>>;
}

impl AsTimerTimestamp for chrono::DateTime<chrono::Utc> {
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        Some(*self)
    }
}

impl AsTimerTimestamp for String {
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(self)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }
}

impl AsTimerTimestamp for str {
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        chrono::DateTime::parse_from_rfc3339(self)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }
}

impl AsTimerTimestamp for &str {
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        (*self).as_timer_timestamp()
    }
}

impl<T: AsTimerTimestamp> AsTimerTimestamp for Option<T> {
    fn as_timer_timestamp(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.as_ref().and_then(AsTimerTimestamp::as_timer_timestamp)
    }
}

/// Action returned from a [`ProcessManager`] reaction to arm timer `T`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetTimer<T>(pub T);

/// Action returned from a [`ProcessManager`] reaction to cancel timer `T`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelTimer<T>(pub T);

/// A named timer declared on a [`ProcessManager`].
///
/// Timers can use either a relative delay (`after(<unit> = <value>)`) or an absolute
/// timestamp field on the process manager state (`at = <field>`). When a [`ProcessManager`]
/// is started via [`EventConsumer::start`](crate::integration::EventConsumer::start),
/// armed timers are persisted with their target UTC deadline and automatically fired in
/// the background (and recovered across restarts).
///
/// # Examples
///
/// ```
/// # use std::{sync::Arc, time::Duration};
/// # use chromatofoor::{
/// #     architecturekit::{
/// #         CancellationToken, Envelope, InMemoryEventStore, Store, Subject, wait_caught_up,
/// #     },
/// #     bounded_context::Command,
/// #     chrono::{DateTime, TimeDelta, Utc},
/// #     common::Identifier,
/// #     integration::{EventConsumer, OnEvent, OnTimer, ProcessManager},
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier>, pub expired: bool }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent {
/// #     OrderCreated { order_id: Identifier, expires_at: DateTime<Utc> },
/// #     OrderExpired { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderCreated {
/// #     fn evolve(&self, mut state: Order) -> Order { state.order_id = Some(self.order_id.clone()); state }
/// # }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderExpired {
/// #     fn evolve(&self, mut state: Order) -> Order { state.expired = true; state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand {
/// #     #[lifecycle(create)]
/// #     CreateOrder { order_id: Identifier, expires_at: DateTime<Utc> },
/// #     ExpireOrder { order_id: Identifier },
/// # }
/// # impl Command<Order> for CreateOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderCreated { order_id: self.order_id, expires_at: self.expires_at }.into()])
/// #     }
/// # }
/// # impl Command<Order> for ExpireOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderExpired { order_id: self.order_id }.into()])
/// #     }
/// # }
/// #[chromatofoor::process_manager(
///     scope = Commerce,
///     correlated_by = event_field(order_id),
///     starts_when = [OrderCreated],
///     emits = [ExpireOrder],
///     timers = [OrderExpiry(at = expires_at)],
///     ends_when = |state: &Self| state.expired,
///     delivery_guarantee = at_least_once,
///     idempotency = self,
/// )]
/// #[derive(Clone, Default)]
/// pub struct OrderExpirationSaga {
///     pub order_id: Option<Identifier>,
///     pub expires_at: Option<DateTime<Utc>>,
///     pub expired: bool,
/// }
///
/// impl OnEvent<OrderCreated> for OrderExpirationSaga {
///     fn on_event(&mut self, event: Envelope<OrderCreated>) -> Vec<Self::Command> {
///         self.order_id = Some(event.data.order_id);
///         self.expires_at = Some(event.data.expires_at);
///         vec![OrderExpiry::set().into()]
///     }
/// }
///
/// impl OnTimer<OrderExpiry> for OrderExpirationSaga {
///     fn on_timer(&mut self) -> Vec<Self::Command> {
///         self.expired = true;
///         vec![ExpireOrder { order_id: self.order_id.clone().unwrap() }.into()]
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// let token = CancellationToken::new();
/// let run = OrderExpirationSaga::default().start(Arc::clone(&store), token.clone());
///
/// let deadline = Utc::now() + TimeDelta::milliseconds(40);
/// store
///     .execute(
///         &CreateOrder::decider(),
///         CreateOrder { order_id: "ord-1".into(), expires_at: deadline },
///     )
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// assert_eq!(
///     OrderExpirationSaga::active_timers(&store, "ord-1").await.unwrap().get("order-expiry"),
///     Some(&deadline)
/// );
///
/// // Once `deadline` passes, the background scheduler automatically fires `OrderExpiry`:
/// tokio::time::sleep(Duration::from_millis(90)).await;
/// wait_caught_up(&[&run]).await.unwrap();
///
/// let order: Order = store.load(&Subject::new("/commerce/ordering/order/ord-1")).await.unwrap();
/// assert!(order.expired);
///
/// // Verify permanent load error handling in the timer task (R2-8) and watcher supervision (R2-10):
/// use chromatofoor::architecturekit;
/// #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, architecturekit::Event)]
/// #[event(type = "io.esdm.process-manager.saved")]
/// struct CorruptSavedEvent {
///     started: String,
/// }
///
/// let perm_deadline = Utc::now() + TimeDelta::milliseconds(60);
/// store
///     .execute(
///         &CreateOrder::decider(),
///         CreateOrder { order_id: "ord-perm".into(), expires_at: perm_deadline },
///     )
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
///
/// // Append a corrupt snapshot event so both the watcher projection (R2-10) and the
/// // scheduled timer task (R2-8) encounter `LoadError::Permanent` and exit cleanly:
/// store
///     .write(
///         vec![chromatofoor::architecturekit::EventOn::new(
///             OrderExpirationSaga::instance_subject("ord-perm"),
///             &CorruptSavedEvent { started: "corrupt-boolean".into() },
///         )],
///         vec![chromatofoor::architecturekit::Precondition::unconditionally()],
///     )
///     .await
///     .unwrap();
/// tokio::time::sleep(Duration::from_millis(90)).await;
/// let order_perm: Order = store.load(&Subject::new("/commerce/ordering/order/ord-perm")).await.unwrap();
/// assert!(!order_perm.expired);
/// # token.cancel();
/// # });
/// ```
pub trait ProcessTimer<P>: Copy + Send + Sync + 'static {
    /// The kebab-case name of this timer (`timers[].name` in ESDM).
    const NAME: &'static str;

    /// The timing schedule of this timer (`after` or `at`).
    const SCHEDULE: TimerSchedule;

    /// Computes the target UTC firing deadline when this timer is armed against `state`.
    fn deadline(state: &P) -> Option<chrono::DateTime<chrono::Utc>>;

    /// Returns an action that arms this timer when returned from a reaction.
    #[must_use]
    fn set(self) -> SetTimer<Self> {
        SetTimer(self)
    }

    /// Returns an action that cancels this timer when returned from a reaction.
    #[must_use]
    fn cancel(self) -> CancelTimer<Self> {
        CancelTimer(self)
    }
}

type TimerKey = (String, String);
type ScheduledTask = (u64, chrono::DateTime<chrono::Utc>, JoinHandle<()>);

struct ProcessTimerWatcher<P> {
    store: Arc<architecturekit::Store>,
    token: architecturekit::CancellationToken,
    tasks: Arc<Mutex<BTreeMap<TimerKey, ScheduledTask>>>,
    next_task_id: std::sync::atomic::AtomicU64,
    _marker: std::marker::PhantomData<fn() -> P>,
}

impl<P: ProcessManager> ProcessTimerWatcher<P> {
    fn sync_instance(
        &self,
        correlation_id: &str,
        instance_subject: &architecturekit::Subject,
        snapshot: &ProcessInstanceState,
    ) {
        let mut guard = self.tasks.lock().unwrap_or_else(PoisonError::into_inner);

        // Cancel any previously scheduled tasks for this instance that were removed,
        // rescheduled to a different deadline, finished, or belong to an ended instance.
        let stale_keys: Vec<TimerKey> = guard
            .iter()
            .filter_map(|((corr, timer), (_, scheduled_at, handle))| {
                if corr != correlation_id {
                    return None;
                }
                let keep = !snapshot.ended
                    && !handle.is_finished()
                    && snapshot.active_timers.get(timer) == Some(scheduled_at);
                (!keep).then(|| (corr.clone(), timer.clone()))
            })
            .collect();

        for key in stale_keys {
            if let Some((_, _, handle)) = guard.remove(&key) {
                handle.abort();
            }
        }

        if snapshot.ended {
            return;
        }

        for (timer_name, &deadline) in &snapshot.active_timers {
            let key = (correlation_id.to_owned(), timer_name.clone());
            if guard.contains_key(&key) {
                continue;
            }

            let task_id = self
                .next_task_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let store = Arc::clone(&self.store);
            let token = self.token.clone();
            let tasks = Arc::clone(&self.tasks);
            let corr_owned = correlation_id.to_owned();
            let timer_owned = timer_name.clone();
            let subject_owned = instance_subject.clone();

            let handle = tokio::spawn(async move {
                const INITIAL_BACKOFF: Duration = Duration::from_millis(50);
                const MAX_BACKOFF: Duration = Duration::from_secs(5);
                const MAX_RETRIES: u32 = 16;

                let remove_self =
                    |tasks: &Mutex<BTreeMap<TimerKey, ScheduledTask>>, corr: &str, timer: &str| {
                        let mut lock = tasks.lock().unwrap_or_else(PoisonError::into_inner);
                        let k = (corr.to_owned(), timer.to_owned());
                        if lock.get(&k).is_some_and(|(id, _, _)| *id == task_id) {
                            lock.remove(&k);
                        }
                    };

                let delay = (deadline - chrono::Utc::now())
                    .to_std()
                    .unwrap_or(Duration::ZERO);

                tokio::select! {
                    () = token.cancelled() => {
                        remove_self(&tasks, &corr_owned, &timer_owned);
                        return;
                    }
                    () = tokio::time::sleep(delay) => {}
                }

                let mut backoff = INITIAL_BACKOFF;
                let mut attempts: u32 = 0;

                loop {
                    match store.load::<ProcessInstanceState>(&subject_owned).await {
                        Ok(current) => {
                            if current.ended
                                || current.active_timers.get(&timer_owned) != Some(&deadline)
                            {
                                break;
                            }
                            remove_self(&tasks, &corr_owned, &timer_owned);
                            let fire_subject = architecturekit::Subject::new(&format!(
                                "{}/timers/{corr_owned}",
                                P::TOPIC
                            ));
                            let fired = ProcessTimerFired {
                                process_manager: P::TOPIC.to_owned(),
                                correlation_id: corr_owned.clone(),
                                timer: timer_owned.clone(),
                                deadline: Some(deadline),
                            };
                            match store
                                .write(
                                    vec![architecturekit::EventOn::new(fire_subject, &fired)],
                                    vec![architecturekit::Precondition::unconditionally()],
                                )
                                .await
                            {
                                Ok(_) => break,
                                Err(architecturekit::WriteError::Permanent(err)) => {
                                    tracing::error!(
                                        process_manager = P::TOPIC,
                                        correlation_id = %corr_owned,
                                        timer = %timer_owned,
                                        error = %err,
                                        "permanent failure writing ProcessTimerFired event; aborting timer"
                                    );
                                    break;
                                }
                                Err(err) => {
                                    attempts = attempts.saturating_add(1);
                                    if attempts >= MAX_RETRIES {
                                        tracing::error!(
                                            process_manager = P::TOPIC,
                                            correlation_id = %corr_owned,
                                            timer = %timer_owned,
                                            attempts,
                                            error = %err,
                                            "exhausted retries writing ProcessTimerFired event; giving up"
                                        );
                                        break;
                                    }
                                    tracing::warn!(
                                        process_manager = P::TOPIC,
                                        correlation_id = %corr_owned,
                                        timer = %timer_owned,
                                        attempt = attempts,
                                        error = %err,
                                        "failed to write ProcessTimerFired event; retrying"
                                    );
                                }
                            }
                        }
                        Err(architecturekit::LoadError::Permanent(err)) => {
                            tracing::error!(
                                process_manager = P::TOPIC,
                                correlation_id = %corr_owned,
                                timer = %timer_owned,
                                error = %err,
                                "permanent failure loading process manager state before firing timer; aborting timer"
                            );
                            break;
                        }
                        Err(err) => {
                            attempts = attempts.saturating_add(1);
                            if attempts >= MAX_RETRIES {
                                tracing::error!(
                                    process_manager = P::TOPIC,
                                    correlation_id = %corr_owned,
                                    timer = %timer_owned,
                                    attempts,
                                    error = %err,
                                    "exhausted retries loading process manager state before firing timer; giving up"
                                );
                                break;
                            }
                            tracing::warn!(
                                process_manager = P::TOPIC,
                                correlation_id = %corr_owned,
                                timer = %timer_owned,
                                attempt = attempts,
                                error = %err,
                                "failed to load process manager state before firing timer; retrying"
                            );
                        }
                    }

                    tokio::select! {
                        () = token.cancelled() => {
                            remove_self(&tasks, &corr_owned, &timer_owned);
                            return;
                        }
                        () = tokio::time::sleep(backoff) => {}
                    }
                    backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
                }

                remove_self(&tasks, &corr_owned, &timer_owned);
            });

            guard.insert(key, (task_id, deadline, handle));
        }
    }
}

impl<P: ProcessManager> architecturekit::Projection for ProcessTimerWatcher<P> {
    type Event = ProcessInstanceEvent;
    type Error = architecturekit::LoadError;

    async fn apply(
        &self,
        event: architecturekit::Envelope<Self::Event>,
    ) -> Result<(), Self::Error> {
        let prefix = format!("/process-managers{}/", P::TOPIC);
        let Some(correlation_id) = event.subject.as_str().strip_prefix(&prefix) else {
            return Ok(());
        };
        let latest: ProcessInstanceState = self.store.load(&event.subject).await?;
        self.sync_instance(correlation_id, &event.subject, &latest);
        Ok(())
    }
}

/// Starts the background timer watcher for [`ProcessManager`] `P`, recovering any
/// persisted active timers from `/process-managers/<domain>/<pm>/...` and automatically
/// writing [`ProcessTimerFired`] events when their deadlines arrive.
pub fn start_process_timers<P: ProcessManager>(
    store: Arc<architecturekit::Store>,
    token: architecturekit::CancellationToken,
) -> architecturekit::ProjectionRun {
    let watcher = Arc::new(ProcessTimerWatcher::<P> {
        store: Arc::clone(&store),
        token: token.clone(),
        tasks: Arc::new(Mutex::new(BTreeMap::new())),
        next_task_id: std::sync::atomic::AtomicU64::new(0),
        _marker: std::marker::PhantomData,
    });
    store.start_projection(
        architecturekit::Subjects::tree(architecturekit::Subject::new(&format!(
            "/process-managers{}",
            P::TOPIC
        ))),
        watcher,
        token,
        architecturekit::ProjectionOptions::named(format!("{}-timers", P::NAME)),
    )
}

/// Supervises a process-manager timer projection run on the active Tokio runtime,
/// logging an error if the timer watcher terminates unexpectedly before cancellation.
pub fn supervise_process_timers<P: ProcessManager>(
    run: architecturekit::ProjectionRun,
    token: architecturekit::CancellationToken,
) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            run.done().await;
            if !token.is_cancelled()
                && let Some(err) = run.error()
            {
                tracing::error!(
                    process_manager = P::TOPIC,
                    error = %err,
                    "process manager timer watcher projection exited with error"
                );
            }
        });
    }
}
