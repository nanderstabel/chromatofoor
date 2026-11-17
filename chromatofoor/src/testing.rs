//! Given-When-Then specification harness modeling [`schemas/given-when-then/v1.yaml`](https://schema.esdm.io/given-when-then/v1).
#![allow(clippy::panic)]
//!
//! Supports all four ESDM `kind: feature` scope variants:
//! 1. **Aggregate (`scope.aggregate`)** — [`given`]:
//!    `Given <events>` $\rightarrow$ `When <command (+ optional actor)>` $\rightarrow$ `Then <events | nothing | rejection>`.
//! 2. **Dynamic Consistency Boundary (`scope.dynamicConsistencyBoundary`)** — [`given`]:
//!    `Given <consulted events>` $\rightarrow$ `When <command (+ optional actor)>` $\rightarrow$ `Then <events | nothing | rejection>`.
//! 3. **Process Manager (`scope.processManager`)** — [`given_process_manager`]:
//!    `Given <preceding events>` $\rightarrow$ `When <event | timer>` $\rightarrow$ `Then <emits, setTimers, cancelTimers, state, ended>`.
//! 4. **Read Model (`scope.readModel`)** — [`given_read_model`]:
//!    `Given <projected events>` $\rightarrow$ `When <query>` $\rightarrow$ `Then <result | readModel>`.
//!
//! # Examples
//!
//! ## 1. Aggregate Feature (`scope.aggregate`)
//!
//! ```
//! # use chromatofoor::{
//! #     bounded_context::{Command, Event},
//! #     common::Identifier,
//! #     testing::given,
//! # };
//! # #[chromatofoor::domain]
//! # pub struct Commerce;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Ordering;
//! #[chromatofoor::actor(scope = Ordering, type = human)]
//! pub struct Customer;
//!
//! #[chromatofoor::actor(scope = Ordering, type = human)]
//! pub struct Guest;
//!
//! #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
//! #[derive(Clone, Default)]
//! pub struct Order {
//!     pub order_id: Option<Identifier>,
//!     pub canceled: bool,
//! }
//!
//! #[chromatofoor::event(scope = Order)]
//! pub enum OrderEvent {
//!     OrderPlaced { order_id: Identifier },
//!     OrderCanceled { order_id: Identifier },
//! }
//! # impl Event<Order> for OrderPlaced {
//! #     fn evolve(&self, mut state: Order) -> Order { state.order_id = Some(self.order_id.clone()); state }
//! # }
//! # impl Event<Order> for OrderCanceled {
//! #     fn evolve(&self, mut state: Order) -> Order { state.canceled = true; state }
//! # }
//!
//! #[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
//! pub enum OrderError {
//!     #[error("order is already canceled")]
//!     AlreadyCanceled,
//! }
//!
//! #[chromatofoor::command(scope = Order, actors = [Customer])]
//! pub enum OrderCommand {
//!     #[lifecycle(create)]
//!     PlaceOrder { order_id: Identifier },
//!     CancelOrder { order_id: Identifier },
//! }
//! # impl Command<Order> for PlaceOrder {
//! #     type Error = OrderError;
//! #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
//! #         Ok(vec![OrderPlaced { order_id: self.order_id }.into()])
//! #     }
//! # }
//! # impl Command<Order> for CancelOrder {
//! #     type Error = OrderError;
//! #     async fn decide(self, state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
//! #         if state.canceled {
//! #             return Err(OrderError::AlreadyCanceled);
//! #         }
//! #         Ok(vec![OrderCanceled { order_id: self.order_id }.into()])
//! #     }
//! # }
//!
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! // Scenario 1: Authorized customer cancels an active order -> emits `OrderCanceled`
//! given::<Order>([OrderPlaced { order_id: "ord-1".into() }.into()])
//!     .when_as::<Customer, _>(CancelOrder { order_id: "ord-1".into() })
//!     .await
//!     .then_events([OrderCanceled { order_id: "ord-1".into() }.into()]);
//!
//! // Scenario 2: Canceling an already-canceled order -> rejected with invariant `AlreadyCanceled`
//! given::<Order>([
//!     OrderPlaced { order_id: "ord-1".into() }.into(),
//!     OrderCanceled { order_id: "ord-1".into() }.into(),
//! ])
//! .when(CancelOrder { order_id: "ord-1".into() })
//! .await
//! .then_rejected(OrderError::AlreadyCanceled);
//!
//! // Scenario 3: Unauthorized actor (`Guest`) -> rejected with reason
//! given::<Order>([])
//!     .when_as::<Guest, _>(PlaceOrder { order_id: "ord-1".into() })
//!     .await
//!     .then_rejected_reason("not authorized");
//! # });
//! ```
//!
//! ## 2. Dynamic Consistency Boundary Feature (`scope.dynamicConsistencyBoundary`)
//!
//! ```
//! # use chromatofoor::{
//! #     bounded_context::{Command, Event},
//! #     common::Identifier,
//! #     testing::given,
//! # };
//! # #[chromatofoor::domain]
//! # pub struct Commerce;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Ordering;
//! # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
//! # #[derive(Clone, Default)]
//! # pub struct Order { pub order_id: Option<Identifier> }
//! # #[chromatofoor::event(scope = Order)]
//! # pub enum OrderEvent { OrderPlaced { order_id: Identifier, customer_id: Identifier } }
//! # impl Event<Order> for OrderPlaced {
//! #     fn evolve(&self, _state: Order) -> Order { Order { order_id: Some(self.order_id.clone()) } }
//! # }
//! #[chromatofoor::event(scope = Ordering)]
//! pub struct CouponRedeemed {
//!     pub customer_id: Identifier,
//!     pub coupon_code: Identifier,
//! }
//!
//! #[chromatofoor::dynamic_consistency_boundary(
//!     scope = Ordering,
//!     identified_by = [command_payload(customer_id), command_payload(coupon_code)],
//!     consults = [OrderPlaced, CouponRedeemed],
//! )]
//! #[derive(Clone, Default)]
//! pub struct CouponRedemption {
//!     pub customers_with_orders: Vec<Identifier>,
//!     pub redeemed: Vec<(Identifier, Identifier)>,
//! }
//! # impl Event<CouponRedemption> for OrderPlaced {
//! #     fn evolve(&self, mut state: CouponRedemption) -> CouponRedemption {
//! #         state.customers_with_orders.push(self.customer_id.clone());
//! #         state
//! #     }
//! # }
//! # impl Event<CouponRedemption> for CouponRedeemed {
//! #     fn evolve(&self, mut state: CouponRedemption) -> CouponRedemption {
//! #         state.redeemed.push((self.customer_id.clone(), self.coupon_code.clone()));
//! #         state
//! #     }
//! # }
//!
//! #[chromatofoor::command(scope = CouponRedemption)]
//! pub enum CouponRedemptionCommand {
//!     RedeemCoupon { customer_id: Identifier, coupon_code: Identifier },
//! }
//! # impl Command<CouponRedemption> for RedeemCoupon {
//! #     type Error = &'static str;
//! #     async fn decide(self, state: &CouponRedemption) -> Result<Vec<CouponRedemptionEvent>, Self::Error> {
//! #         if !state.customers_with_orders.contains(&self.customer_id) {
//! #             return Err("customer has no orders");
//! #         }
//! #         if state.redeemed.contains(&(self.customer_id.clone(), self.coupon_code.clone())) {
//! #             return Err("coupon already redeemed");
//! #         }
//! #         Ok(vec![CouponRedeemed { customer_id: self.customer_id, coupon_code: self.coupon_code }.into()])
//! #     }
//! # }
//!
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! given::<CouponRedemption>([
//!     OrderPlaced { order_id: "ord-1".into(), customer_id: "cust-1".into() }.into(),
//! ])
//! .when(RedeemCoupon { customer_id: "cust-1".into(), coupon_code: "WELCOME10".into() })
//! .await
//! .then_events([
//!     CouponRedeemed { customer_id: "cust-1".into(), coupon_code: "WELCOME10".into() }.into(),
//! ]);
//! # });
//! ```
//!
//! ## 3. Process Manager Feature (`scope.processManager`)
//!
//! ```
//! # use chromatofoor::{
//! #     architecturekit::Envelope,
//! #     bounded_context::{Command, Event},
//! #     common::Identifier,
//! #     integration::{OnEvent, OnTimer},
//! #     testing::given_process_manager,
//! # };
//! # #[chromatofoor::domain]
//! # pub struct Commerce;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Ordering;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Billing;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Shipping;
//! # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
//! # #[derive(Clone, Default)]
//! # pub struct Order { pub order_id: Option<Identifier> }
//! # #[chromatofoor::event(scope = Order)]
//! # pub enum OrderEvent { OrderPlaced { order_id: Identifier }, OrderCanceled { order_id: Identifier } }
//! # impl Event<Order> for OrderPlaced { fn evolve(&self, s: Order) -> Order { s } }
//! # impl Event<Order> for OrderCanceled { fn evolve(&self, s: Order) -> Order { s } }
//! # #[chromatofoor::command(scope = Order)]
//! # pub enum OrderCommand { CancelOrder { order_id: Identifier } }
//! # impl Command<Order> for CancelOrder {
//! #     type Error = &'static str;
//! #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
//! # }
//! # #[chromatofoor::event(scope = Billing)]
//! # pub struct PaymentCaptured { pub order_id: Identifier }
//! # #[chromatofoor::aggregate(scope = Shipping, identified_by = state(order_id))]
//! # #[derive(Clone, Default)]
//! # pub struct Shipment { pub order_id: Option<Identifier> }
//! # #[chromatofoor::event(scope = Shipment)]
//! # pub enum ShipmentEvent { ShipmentScheduled { order_id: Identifier } }
//! # impl Event<Shipment> for ShipmentScheduled { fn evolve(&self, s: Shipment) -> Shipment { s } }
//! # #[chromatofoor::command(scope = Shipment)]
//! # pub enum ShipmentCommand {
//! #     #[lifecycle(create)]
//! #     ScheduleShipment { order_id: Identifier },
//! # }
//! # impl Command<Shipment> for ScheduleShipment {
//! #     type Error = &'static str;
//! #     async fn decide(self, _: &Shipment) -> Result<Vec<ShipmentEvent>, Self::Error> { Ok(vec![]) }
//! # }
//! #[chromatofoor::process_manager(
//!     scope = Commerce,
//!     correlated_by = event_field(order_id),
//!     starts_when = [OrderPlaced],
//!     handles = [PaymentCaptured],
//!     emits = [ScheduleShipment, CancelOrder],
//!     timers = [PaymentTimeout(after(minutes = 15))],
//!     ends_when = |state: &Self| state.shipped || state.canceled,
//!     delivery_guarantee = at_least_once,
//!     idempotency = self,
//! )]
//! #[derive(Clone, Debug, Default, PartialEq)]
//! pub struct OrderFulfillment {
//!     pub order_id: Option<Identifier>,
//!     pub shipped: bool,
//!     pub canceled: bool,
//! }
//!
//! impl OnEvent<OrderPlaced> for OrderFulfillment {
//!     fn on_event(&mut self, event: Envelope<OrderPlaced>) -> Vec<Self::Command> {
//!         self.order_id = Some(event.data.order_id);
//!         vec![PaymentTimeout::set().into()]
//!     }
//! }
//!
//! impl OnEvent<PaymentCaptured> for OrderFulfillment {
//!     fn on_event(&mut self, event: Envelope<PaymentCaptured>) -> Vec<Self::Command> {
//!         self.shipped = true;
//!         vec![
//!             PaymentTimeout::cancel().into(),
//!             ScheduleShipment { order_id: event.data.order_id }.into(),
//!         ]
//!     }
//! }
//!
//! impl OnTimer<PaymentTimeout> for OrderFulfillment {
//!     fn on_timer(&mut self) -> Vec<Self::Command> {
//!         self.canceled = true;
//!         vec![CancelOrder { order_id: self.order_id.clone().unwrap() }.into()]
//!     }
//! }
//!
//! // 1. When `PaymentCaptured` arrives after `OrderPlaced`:
//! given_process_manager::<OrderFulfillment>([
//!     OrderPlaced { order_id: "ord-1".into() }.into(),
//! ])
//! .when_event(PaymentCaptured { order_id: "ord-1".into() })
//! .then_emits([ScheduleShipment { order_id: "ord-1".into() }.into()])
//! .then_cancel_timer::<PaymentTimeout>()
//! .then_state(&OrderFulfillment {
//!     order_id: Some("ord-1".into()),
//!     shipped: true,
//!     canceled: false,
//! })
//! .then_ended();
//!
//! // 2. When `PaymentTimeout` fires after `OrderPlaced`:
//! given_process_manager::<OrderFulfillment>([
//!     OrderPlaced { order_id: "ord-1".into() }.into(),
//! ])
//! .when_timer::<PaymentTimeout>()
//! .then_emits([CancelOrder { order_id: "ord-1".into() }.into()])
//! .then_ended();
//!
//! // 3. Unarmed or cancelled timers do not fire:
//! given_process_manager::<OrderFulfillment>([])
//!     .when_timer::<PaymentTimeout>()
//!     .then_emits([]);
//! ```
//!
//! ## 4. Read Model Feature (`scope.readModel`)
//!
//! ```
//! # use chromatofoor::{
//! #     architecturekit::{Envelope, InMemoryView, KeyedView, ViewError},
//! #     bounded_context::{Project, Query},
//! #     testing::given_read_model,
//! # };
//! # #[chromatofoor::domain]
//! # pub struct Commerce;
//! # #[chromatofoor::bounded_context(scope = Commerce)]
//! # pub struct Ordering;
//! #[chromatofoor::event(scope = Ordering)]
//! pub struct OrderPlaced {
//!     pub order_id: String,
//!     pub total_cents: u64,
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
//!             .insert(&event, (event.data.order_id.clone(), event.data.total_cents))?;
//!         Ok(())
//!     }
//! }
//!
//! #[chromatofoor::query(scope = Ordering)]
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
//! let rm = OrderTotals {
//!     totals: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
//! };
//!
//! given_read_model(
//!     rm,
//!     [OrderPlaced { order_id: "ord-1".to_owned(), total_cents: 4200 }.into()],
//! )
//! .await
//! .when(OrderTotalById { order_id: "ord-1".to_owned() })
//! .await
//! .then_result(&Some(4200));
//! # });
//! ```

use std::{borrow::Cow, collections::BTreeSet, fmt};

use architecturekit::{
    Envelope, Events, Precondition, Projected, Revision, Subject, check_preconditions,
    testing::{StoredEvent, project, stored_events},
};
use serde_json::Value;

use crate::{
    bounded_context::{Actor, Command, Query, ReadModel, command::CommandScope},
    common::{IdSource, resolve_subject},
    integration::{ProcessCommand, ProcessManager, ProcessTimer, ProcessTimerFired},
};

/// Asserts that every key/element in `expected` is present and equal in `actual`.
///
/// Used when checking partial state or read-model expectations from Given-When-Then specs.
#[track_caller]
pub fn assert_json_subset(expected: &Value, actual: &Value, path: &str) {
    match expected {
        Value::Object(exp_map) => {
            let Value::Object(act_map) = actual else {
                panic!("JSON subset mismatch at '{path}': expected Object, found {actual:?}");
            };
            for (k, v) in exp_map {
                let next_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                assert!(
                    act_map.contains_key(k),
                    "JSON subset mismatch at '{path}': missing key '{k}' in {act_map:?}"
                );
                assert_json_subset(v, &act_map[k], &next_path);
            }
        }
        Value::Array(exp_arr) => {
            let Value::Array(act_arr) = actual else {
                panic!("JSON subset mismatch at '{path}': expected Array, found {actual:?}");
            };
            assert_eq!(
                exp_arr.len(),
                act_arr.len(),
                "JSON subset mismatch at '{path}': array length mismatch (expected {}, got {})",
                exp_arr.len(),
                act_arr.len()
            );
            for (idx, (exp_val, act_val)) in exp_arr.iter().zip(act_arr.iter()).enumerate() {
                assert_json_subset(exp_val, act_val, &format!("{path}[{idx}]"));
            }
        }
        _ => {
            assert_eq!(
                expected, actual,
                "JSON subset mismatch at '{path}': expected {expected}, got {actual}"
            );
        }
    }
}

// ============================================================================
// 1 & 2. Consistency Unit Fixture (`Aggregate` & `DynamicConsistencyBoundary`)
// ============================================================================

/// Builds a Given-When-Then fixture for an [`Aggregate`](crate::bounded_context::Aggregate) or
/// [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary) `S` from
/// preceding `history` events.
#[must_use]
#[track_caller]
pub fn given<S: CommandScope>(history: impl IntoIterator<Item = S::Event>) -> UnitFixture<S> {
    let history: Vec<S::Event> = history.into_iter().collect();
    for (idx, event) in history.iter().enumerate() {
        if let Err(err) = event.to_data() {
            panic!(
                "given: event {idx} ({:?}) could not be encoded as JSON: {err}",
                event.event_type()
            );
        }
    }
    UnitFixture { history }
}

/// Preceding event history for a consistency unit (`Aggregate` or `DynamicConsistencyBoundary`).
pub struct UnitFixture<S: CommandScope> {
    history: Vec<S::Event>,
}

impl<S: CommandScope> fmt::Debug for UnitFixture<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitFixture")
            .field("history_len", &self.history.len())
            .finish()
    }
}

enum UnitFailure<E> {
    Domain(E),
    Unauthorized {
        actor: String,
        command: &'static str,
    },
    Precondition(String),
    Encoding(String),
}

impl<E: fmt::Display> fmt::Display for UnitFailure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(err) => fmt::Display::fmt(err, f),
            Self::Unauthorized { actor, command } => {
                write!(
                    f,
                    "actor `{actor}` is not authorized to execute `{command}`"
                )
            }
            Self::Precondition(msg) | Self::Encoding(msg) => f.write_str(msg),
        }
    }
}

impl<S: CommandScope> UnitFixture<S> {
    /// Executes `command` against the state folded from `given` without actor restriction.
    pub async fn when<C: Command<S>>(self, command: C) -> UnitDecision<S, C::Error> {
        self.execute_when(None, command).await
    }

    /// Executes `command` as typed [`Actor`] `Act`, verifying [`Authorized::allows::<Act>()`](crate::bounded_context::Authorized::allows)
    /// before running `Command::decide`.
    pub async fn when_as<Act: Actor, C: Command<S>>(self, command: C) -> UnitDecision<S, C::Error> {
        let authorized = command.is_authorized_for_actor::<Act>();
        self.execute_when_checked(Some((Act::NAME, authorized)), command)
            .await
    }

    /// Executes `command` as `actor` (kebab-case name), verifying
    /// [`Authorized::is_allowed_actor`](crate::bounded_context::Authorized::is_allowed_actor) before running `Command::decide`.
    pub async fn when_by<C: Command<S>>(
        self,
        actor: &str,
        command: C,
    ) -> UnitDecision<S, C::Error> {
        let authorized = command.is_authorized_for(actor);
        self.execute_when_checked(Some((actor, authorized)), command)
            .await
    }

    async fn execute_when<C: Command<S>>(
        self,
        actor: Option<&str>,
        command: C,
    ) -> UnitDecision<S, C::Error> {
        let checked = actor.map(|a| (a, command.is_authorized_for(a)));
        self.execute_when_checked(checked, command).await
    }

    async fn execute_when_checked<C: Command<S>>(
        self,
        actor: Option<(&str, bool)>,
        command: C,
    ) -> UnitDecision<S, C::Error> {
        let subject = command.subject();
        let preconditions = command.preconditions();

        let mut stored_history: Vec<architecturekit::eventsourcingdb::Event> =
            Vec::with_capacity(self.history.len());
        let mut last_event_subject = subject.clone();

        // Fold history into `S`, honoring subject identity for Aggregates and `Command::matches` for DCBs.
        let mut state = S::default();
        for (idx, ev) in self.history.into_iter().enumerate() {
            let ev_subject = resolve_event_subject::<S>(&ev, &last_event_subject);
            last_event_subject = ev_subject.clone();
            let stored: architecturekit::eventsourcingdb::Event =
                StoredEvent::new(ev_subject, u32::try_from(idx).unwrap_or(u32::MAX), &ev).into();
            match <S::Event as Projected>::project(&stored, S::upcasters()) {
                Ok(envelopes) => {
                    let same_subject = stored.subject() == subject.as_str();
                    for env in envelopes {
                        let should_fold = if S::KIND == "aggregate" {
                            same_subject && command.matches(&env)
                        } else {
                            command.matches(&env)
                        };
                        if should_fold {
                            state = state.evolve(&env.data);
                        }
                    }
                }
                Err(err) => {
                    return UnitDecision {
                        state,
                        preconditions,
                        result: Err(UnitFailure::Precondition(err.to_string())),
                    };
                }
            }
            stored_history.push(stored);
        }

        if let Some((actor_name, false)) = actor {
            return UnitDecision {
                state,
                preconditions,
                result: Err(UnitFailure::Unauthorized {
                    actor: actor_name.to_owned(),
                    command: C::NAME,
                }),
            };
        }

        if let Err(refusal) = check_preconditions(&command) {
            return UnitDecision {
                state,
                preconditions,
                result: Err(UnitFailure::Precondition(refusal.to_string())),
            };
        }

        if let Err(err) = command.validate_payload() {
            return UnitDecision {
                state,
                preconditions,
                result: Err(UnitFailure::Domain(err)),
            };
        }

        let result = match command.decide(&state).await {
            Ok(events) => {
                if let Err(refusal) = evaluate_unit_preconditions(&preconditions, &stored_history) {
                    return UnitDecision {
                        state,
                        preconditions,
                        result: Err(UnitFailure::Precondition(refusal)),
                    };
                }
                let mut encoded = Vec::with_capacity(events.len());
                for ev in &events {
                    match ev.to_data() {
                        Ok(data) => encoded.push(data),
                        Err(err) => {
                            return UnitDecision {
                                state,
                                preconditions,
                                result: Err(UnitFailure::Encoding(err.to_string())),
                            };
                        }
                    }
                }
                Ok((events, encoded))
            }
            Err(err) => Err(UnitFailure::Domain(err)),
        };

        UnitDecision {
            state,
            preconditions,
            result,
        }
    }
}

type UnitOutcome<S, E> =
    Result<(Vec<<S as architecturekit::State>::Event>, Vec<Value>), UnitFailure<E>>;

/// Outcome of executing a command on a consistency unit (`Aggregate` or `DynamicConsistencyBoundary`).
pub struct UnitDecision<S: CommandScope, E> {
    state: S,
    preconditions: Vec<Precondition>,
    result: UnitOutcome<S, E>,
}

impl<S: CommandScope, E: fmt::Debug + fmt::Display> fmt::Debug for UnitDecision<S, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnitDecision")
            .field("preconditions", &self.preconditions)
            .finish_non_exhaustive()
    }
}

impl<S: CommandScope, E: fmt::Debug + fmt::Display> UnitDecision<S, E> {
    /// Asserts that the command succeeded and emitted `expected` events in order (`then.events`).
    #[track_caller]
    pub fn then_events(&self, expected: impl IntoIterator<Item = S::Event>) -> &Self {
        let (events, data) = match &self.result {
            Ok(pair) => pair,
            Err(failure) => panic!("expected events, but command was rejected: {failure}"),
        };
        let expected: Vec<S::Event> = expected.into_iter().collect();
        assert_eq!(
            events.len(),
            expected.len(),
            "expected {} event(s), got {}",
            expected.len(),
            events.len()
        );

        for (idx, (wanted, (got, got_data))) in
            expected.iter().zip(events.iter().zip(data)).enumerate()
        {
            assert_eq!(
                got.event_type(),
                wanted.event_type(),
                "event {idx}: got type {:?}, want {:?}",
                got.event_type(),
                wanted.event_type()
            );
            let wanted_data = wanted
                .to_data()
                .unwrap_or_else(|e| panic!("event {idx}: failed to serialize expected event: {e}"));
            assert_eq!(
                *got_data, wanted_data,
                "event {idx}: got payload {got_data}, want {wanted_data}"
            );
        }
        self
    }

    /// Asserts that the command succeeded and emitted zero events (`then: { events: [] }`).
    #[track_caller]
    pub fn then_nothing(&self) -> &Self {
        let (events, _) = match &self.result {
            Ok(pair) => pair,
            Err(failure) => panic!("expected no events, but command was rejected: {failure}"),
        };
        assert!(
            events.is_empty(),
            "expected no events, got {} event(s)",
            events.len()
        );
        self
    }

    /// Asserts that the command was rejected by the decider with `expected` (`then.rejection.invariant`).
    #[track_caller]
    #[allow(clippy::needless_pass_by_value)]
    pub fn then_rejected(&self, expected: E) -> &Self
    where
        E: PartialEq,
    {
        match &self.result {
            Err(UnitFailure::Domain(actual)) if *actual == expected => {}
            Err(other) => panic!("expected rejection {expected:?}, got: {other}"),
            Ok((events, _)) => panic!(
                "expected rejection {expected:?}, but command succeeded with {} event(s)",
                events.len()
            ),
        }
        self
    }

    /// Asserts that the command was rejected (by the decider, actor authorization, or preconditions)
    /// with a message containing `expected_substring` (`then.rejection.reason`).
    #[track_caller]
    pub fn then_rejected_reason(&self, expected_substring: &str) -> &Self {
        match &self.result {
            Err(failure) => {
                let message = failure.to_string();
                assert!(
                    message.contains(expected_substring),
                    "expected rejection reason containing {expected_substring:?}, got {message:?}"
                );
            }
            Ok((events, _)) => panic!(
                "expected rejection containing {expected_substring:?}, but command succeeded with {} event(s)",
                events.len()
            ),
        }
        self
    }

    /// Inspects the folded state `S` on which the command was decided.
    pub fn then_state(&self, check: impl FnOnce(&S)) -> &Self {
        check(&self.state);
        self
    }

    /// Asserts that the command declared `expected` preconditions.
    #[track_caller]
    pub fn then_preconditions(&self, expected: impl IntoIterator<Item = Precondition>) -> &Self {
        let expected: Vec<Precondition> = expected.into_iter().collect();
        assert_eq!(self.preconditions, expected);
        self
    }
}

fn normalize_field_key(key: &str) -> String {
    key.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn extract_json_id_field(map: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    let target = normalize_field_key(field);
    map.iter().find_map(|(k, v)| {
        if k == field || normalize_field_key(k) == target {
            match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn resolve_event_subject<S: CommandScope>(ev: &S::Event, fallback: &Subject) -> Subject {
    if S::KIND != "aggregate" || S::IDENTIFIED_BY.is_empty() {
        return fallback.clone();
    }
    let Ok(Value::Object(map)) = ev.to_data() else {
        return fallback.clone();
    };

    let mut pairs: Vec<(&'static str, Option<Cow<'_, str>>)> =
        Vec::with_capacity(S::IDENTIFIED_BY.len());
    let mut found_any = false;

    for source in S::IDENTIFIED_BY {
        match *source {
            IdSource::State(field) | IdSource::CommandPayload(field) | IdSource::Schema(field) => {
                if let Some(val) = extract_json_id_field(&map, field) {
                    found_any = true;
                    pairs.push((field, Some(Cow::Owned(val))));
                } else {
                    pairs.push((field, None));
                }
            }
            IdSource::Static(_) => {}
            IdSource::Generated(_) => {
                if let Some(val) = extract_json_id_field(&map, "id") {
                    found_any = true;
                    pairs.push(("id", Some(Cow::Owned(val))));
                }
            }
        }
    }

    if found_any {
        resolve_subject::<S>(&pairs)
    } else {
        fallback.clone()
    }
}

fn subject_matches_precondition(event_subject: &str, target: &str, recursive: bool) -> bool {
    if event_subject == target {
        return true;
    }
    if recursive {
        if target == "/" {
            return true;
        }
        let prefix = format!("{}/", target.trim_end_matches('/'));
        return event_subject.starts_with(&prefix);
    }
    false
}

fn evaluate_unit_preconditions(
    preconditions: &[Precondition],
    events: &[architecturekit::eventsourcingdb::Event],
) -> Result<(), String> {
    for precondition in preconditions {
        let Some(db_precondition) = precondition.database() else {
            continue;
        };
        match db_precondition {
            architecturekit::eventsourcingdb::Precondition::IsSubjectPristine {
                subject,
                options,
                ..
            } => {
                let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
                if events
                    .iter()
                    .any(|ev| subject_matches_precondition(ev.subject(), subject, recursive))
                {
                    return Err(format!(
                        "precondition isSubjectPristine does not hold for subject {subject:?}"
                    ));
                }
            }
            architecturekit::eventsourcingdb::Precondition::IsSubjectPopulated {
                subject,
                options,
                ..
            } => {
                let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
                if !events
                    .iter()
                    .any(|ev| subject_matches_precondition(ev.subject(), subject, recursive))
                {
                    return Err(format!(
                        "precondition isSubjectPopulated does not hold for subject {subject:?}"
                    ));
                }
            }
            architecturekit::eventsourcingdb::Precondition::IsSubjectOnEventId {
                subject,
                event_id,
                options,
                ..
            } => {
                let recursive = options.as_ref().is_some_and(|opts| opts.recursive);
                let latest = events
                    .iter()
                    .rev()
                    .find(|ev| subject_matches_precondition(ev.subject(), subject, recursive));
                if latest.is_none_or(|ev| ev.id() != event_id) {
                    return Err(format!(
                        "precondition isSubjectOnEventId does not hold for subject {subject:?}"
                    ));
                }
            }
            architecturekit::eventsourcingdb::Precondition::IsEventQLQueryTrue { .. } => {}
        }
    }
    Ok(())
}

// ============================================================================
// 3. Process Manager Fixture (`scope.processManager`)
// ============================================================================

/// Builds an in-memory Given-When-Then fixture for [`ProcessManager`] `P` by replaying `history`.
#[must_use]
pub fn given_process_manager<P: ProcessManager>(
    history: impl IntoIterator<Item = P::Event>,
) -> ProcessFixture<P> {
    let mut state: Option<P> = None;
    let mut ended = false;
    let mut active_timers: BTreeSet<&'static str> = BTreeSet::new();
    let mut next_rev: u32 = 0;

    for ev in history {
        let env = make_envelope(P::TOPIC, next_rev, ev);
        next_rev = next_rev.saturating_add(1);
        let _ = P::step(&mut state, &mut ended, &mut active_timers, env);
    }

    ProcessFixture {
        state,
        ended,
        active_timers,
        next_rev,
    }
}

/// Preceding state of a [`ProcessManager`] instance built by [`given_process_manager`].
pub struct ProcessFixture<P: ProcessManager> {
    state: Option<P>,
    ended: bool,
    active_timers: BTreeSet<&'static str>,
    next_rev: u32,
}

impl<P: ProcessManager + fmt::Debug> fmt::Debug for ProcessFixture<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessFixture")
            .field("state", &self.state)
            .field("ended", &self.ended)
            .field("active_timers", &self.active_timers)
            .field("next_rev", &self.next_rev)
            .finish()
    }
}

impl<P: ProcessManager> ProcessFixture<P> {
    /// Triggers the process manager with an incoming domain `event` (`when` with `eventReference`).
    #[must_use]
    pub fn when_event(mut self, event: impl Into<P::Event>) -> ProcessDecision<P> {
        let env = make_envelope(P::TOPIC, self.next_rev, event.into());
        let actions = P::step(
            &mut self.state,
            &mut self.ended,
            &mut self.active_timers,
            env,
        );
        ProcessDecision::from_actions(self.state, self.ended, actions)
    }

    /// Triggers the process manager by firing typed [`ProcessTimer`] `T` (`when: { timer }`).
    #[must_use]
    pub fn when_timer<T: ProcessTimer<P>>(self) -> ProcessDecision<P>
    where
        P::Event: From<ProcessTimerFired>,
    {
        self.when_timer_named(T::NAME)
    }

    /// Triggers the process manager by firing the timer with kebab-case `timer_name` (`when: { timer }`).
    #[must_use]
    pub fn when_timer_named(mut self, timer_name: &'static str) -> ProcessDecision<P>
    where
        P::Event: From<ProcessTimerFired>,
    {
        let fired = ProcessTimerFired {
            process_manager: P::TOPIC.to_owned(),
            correlation_id: "test".to_owned(),
            timer: timer_name.to_owned(),
            deadline: None,
        };
        let env = make_envelope(P::TOPIC, self.next_rev, P::Event::from(fired));
        let actions = P::step(
            &mut self.state,
            &mut self.ended,
            &mut self.active_timers,
            env,
        );
        ProcessDecision::from_actions(self.state, self.ended, actions)
    }
}

/// Outcome of a [`ProcessManager`] reaction (`processManagerThen` in `given-when-then/v1.yaml`).
#[derive(Debug)]
pub struct ProcessDecision<P: ProcessManager> {
    state: P,
    ended: bool,
    emitted: Vec<P::Command>,
    set_timers: Vec<&'static str>,
    cancel_timers: Vec<&'static str>,
}

impl<P: ProcessManager> ProcessDecision<P> {
    fn from_actions(state: Option<P>, ended: bool, actions: Vec<P::Command>) -> Self {
        let mut emitted = Vec::new();
        let mut set_timers = Vec::new();
        let mut cancel_timers = Vec::new();

        for action in actions {
            if let Some(t) = action.as_set_timer() {
                set_timers.push(t);
            } else if let Some(t) = action.as_cancel_timer() {
                cancel_timers.push(t);
            } else {
                emitted.push(action);
            }
        }

        Self {
            state: state.unwrap_or_default(),
            ended,
            emitted,
            set_timers,
            cancel_timers,
        }
    }

    /// Asserts that the reaction emitted `expected` domain commands in order (`then.emits`).
    #[track_caller]
    pub fn then_emits(&self, expected: impl IntoIterator<Item = P::Command>) -> &Self {
        let expected: Vec<P::Command> = expected
            .into_iter()
            .filter(|cmd| cmd.as_set_timer().is_none() && cmd.as_cancel_timer().is_none())
            .collect();
        assert_eq!(
            self.emitted, expected,
            "emitted commands mismatch: got {:?}, want {:?}",
            self.emitted, expected
        );
        self
    }

    /// Asserts that the reaction emitted a command matching `(bounded_context, target, command)`
    /// and optional `data` JSON subset (`scopedCommandReference` in `given-when-then/v1.yaml`).
    #[track_caller]
    pub fn then_emits_command(
        &self,
        bounded_context: &str,
        target: &str,
        command: &str,
        expected_data: Option<&Value>,
    ) -> &Self {
        let found = self.emitted.iter().find_map(|action| {
            let (bc, tgt, cmd, data) = action.as_emitted_command()?;
            (bc == bounded_context && tgt == target && cmd == command).then_some(data)
        });
        let Some(actual_data) = found else {
            panic!(
                "expected emitted command `{bounded_context}.{target}.{command}`, got {:?}",
                self.emitted
            );
        };
        if let Some(exp) = expected_data {
            assert_json_subset(exp, &actual_data, command);
        }
        self
    }

    /// Asserts that timer `T` was armed during this reaction (`then.setTimers`).
    #[track_caller]
    pub fn then_set_timer<T: ProcessTimer<P>>(&self) -> &Self {
        assert!(
            self.set_timers.contains(&T::NAME),
            "expected timer {:?} to be set, but setTimers was {:?}",
            T::NAME,
            self.set_timers
        );
        self
    }

    /// Asserts that `expected` timer names were armed during this reaction (`then.setTimers`).
    #[track_caller]
    pub fn then_set_timers<S: AsRef<str>>(&self, expected: impl IntoIterator<Item = S>) -> &Self {
        let expected: Vec<String> = expected
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        let actual: Vec<String> = self.set_timers.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(actual, expected, "setTimers mismatch");
        self
    }

    /// Asserts that timer `T` was cancelled during this reaction (`then.cancelTimers`).
    #[track_caller]
    pub fn then_cancel_timer<T: ProcessTimer<P>>(&self) -> &Self {
        assert!(
            self.cancel_timers.contains(&T::NAME),
            "expected timer {:?} to be cancelled, but cancelTimers was {:?}",
            T::NAME,
            self.cancel_timers
        );
        self
    }

    /// Asserts that `expected` timer names were cancelled during this reaction (`then.cancelTimers`).
    #[track_caller]
    pub fn then_cancel_timers<S: AsRef<str>>(
        &self,
        expected: impl IntoIterator<Item = S>,
    ) -> &Self {
        let expected: Vec<String> = expected
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        let actual: Vec<String> = self.cancel_timers.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(actual, expected, "cancelTimers mismatch");
        self
    }

    /// Asserts that the resulting instance state equals `expected` (`then.state`).
    #[track_caller]
    pub fn then_state(&self, expected: &P) -> &Self
    where
        P: PartialEq + fmt::Debug,
    {
        assert_eq!(&self.state, expected, "process manager state mismatch");
        self
    }

    /// Asserts that the serialized instance state contains `expected` as a JSON subset (`then.state`).
    #[track_caller]
    pub fn then_state_json(&self, expected: &Value) -> &Self {
        let actual = serde_json::to_value(&self.state)
            .unwrap_or_else(|e| panic!("failed to serialize process manager state: {e}"));
        assert_json_subset(expected, &actual, "state");
        self
    }

    /// Asserts that the process instance reached its terminal `ends_when` condition (`then.ended: true`).
    #[track_caller]
    pub fn then_ended(&self) -> &Self {
        assert!(
            self.ended,
            "expected process manager instance to have ended (`ended: true`), but `is_ended()` was false"
        );
        self
    }
}

#[derive(serde::Serialize, serde::Deserialize, architecturekit::Event)]
#[event(type = "io.esdm.test.envelope")]
struct EnvelopeSeed {}

#[derive(architecturekit::Events)]
enum EnvelopeSeedEvents {
    Seed(EnvelopeSeed),
}

fn make_envelope<E>(subject_str: &str, rev: u32, data: E) -> Envelope<E> {
    Envelope::new(
        Subject::new(subject_str),
        Revision::from(rev),
        EnvelopeSeedEvents::Seed(EnvelopeSeed {}),
    )
    .map(|_| data)
}

// ============================================================================
// 4. Read Model Fixture (`scope.readModel`)
// ============================================================================

const DEFAULT_READ_MODEL_SUBJECT_ID: &str = "00000000-0000-0000-0000-000000000001";

/// Projects `history` into `read_model` under `<R::TOPIC>/00000000-0000-0000-0000-000000000001`
/// using [`architecturekit::testing::project`] and returns a [`ReadModelFixture`] for asserting on
/// the materialized view or executing a [`Query`].
///
/// Use [`given_read_model_with_id`] when a subject-keyed projection expects a specific trailing
/// subject segment.
///
/// # Panics
///
/// Panics if projecting any event in `history` fails.
pub async fn given_read_model<R: ReadModel>(
    read_model: R,
    history: impl IntoIterator<Item = R::Event>,
) -> ReadModelFixture<R>
where
    R::Event: Events,
{
    given_read_model_with_id(read_model, DEFAULT_READ_MODEL_SUBJECT_ID, history).await
}

/// Projects `history` into `read_model` under `<R::TOPIC>/<instance_id>` using
/// [`architecturekit::testing::project`] and returns a [`ReadModelFixture`].
///
/// # Panics
///
/// Panics if projecting any event in `history` fails.
pub async fn given_read_model_with_id<R: ReadModel>(
    read_model: R,
    instance_id: &str,
    history: impl IntoIterator<Item = R::Event>,
) -> ReadModelFixture<R>
where
    R::Event: Events,
{
    let trimmed = instance_id.trim_matches('/');
    let segment = if trimmed.is_empty() {
        DEFAULT_READ_MODEL_SUBJECT_ID
    } else {
        trimmed
    };
    let subject = Subject::new(&format!("{}/{segment}", R::TOPIC));
    let stored = stored_events(&subject, history);
    if let Err(err) = project(&read_model, &stored).await {
        panic!(
            "given_read_model: failed to project history into `{}`: {err}",
            R::NAME
        );
    }
    ReadModelFixture { read_model }
}

/// Materialized [`ReadModel`] state built by [`given_read_model`].
pub struct ReadModelFixture<R: ReadModel> {
    read_model: R,
}

impl<R: ReadModel + fmt::Debug> fmt::Debug for ReadModelFixture<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadModelFixture")
            .field("read_model", &self.read_model)
            .finish()
    }
}

impl<R: ReadModel> ReadModelFixture<R> {
    /// Executes `query` against the projected read model (`when: { query, parameters }`).
    pub async fn when<Q: Query<R>>(self, query: Q) -> ReadModelDecision<R, Q::Output, Q::Error> {
        self.execute_when(None, query).await
    }

    /// Executes `query` as typed [`Actor`] `Act`, checking [`Authorized::allows::<Act>()`](crate::bounded_context::Authorized::allows).
    pub async fn when_as<Act: Actor, Q: Query<R>>(
        self,
        query: Q,
    ) -> ReadModelDecision<R, Q::Output, Q::Error> {
        let authorized = query.is_authorized_for_actor::<Act>();
        self.execute_when_checked(Some((Act::NAME, authorized)), query)
            .await
    }

    /// Executes `query` as `actor` (kebab-case name), checking [`Authorized::is_allowed_actor`](crate::bounded_context::Authorized::is_allowed_actor).
    pub async fn when_by<Q: Query<R>>(
        self,
        actor: &str,
        query: Q,
    ) -> ReadModelDecision<R, Q::Output, Q::Error> {
        let authorized = query.is_authorized_for(actor);
        self.execute_when_checked(Some((actor, authorized)), query)
            .await
    }

    async fn execute_when<Q: Query<R>>(
        self,
        actor: Option<&str>,
        query: Q,
    ) -> ReadModelDecision<R, Q::Output, Q::Error> {
        let checked = actor.map(|a| (a, query.is_authorized_for(a)));
        self.execute_when_checked(checked, query).await
    }

    async fn execute_when_checked<Q: Query<R>>(
        self,
        actor: Option<(&str, bool)>,
        query: Q,
    ) -> ReadModelDecision<R, Q::Output, Q::Error> {
        if let Some((actor_name, false)) = actor {
            return ReadModelDecision {
                read_model: self.read_model,
                result: Err(UnitFailure::Unauthorized {
                    actor: actor_name.to_owned(),
                    command: Q::NAME,
                }),
            };
        }
        let result = query
            .execute(&self.read_model)
            .await
            .map_err(UnitFailure::Domain);
        ReadModelDecision {
            read_model: self.read_model,
            result,
        }
    }

    /// Asserts directly on the materialized read model (`then.readModel`).
    #[track_caller]
    pub fn then_read_model(&self, expected: &R) -> &Self
    where
        R: PartialEq + fmt::Debug,
    {
        assert_eq!(&self.read_model, expected, "read model mismatch");
        self
    }

    /// Asserts that the serialized read model contains `expected` as a JSON subset (`then.readModel`).
    #[track_caller]
    pub fn then_read_model_json(&self, expected: &Value) -> &Self
    where
        R: serde::Serialize,
    {
        let actual = serde_json::to_value(&self.read_model)
            .unwrap_or_else(|e| panic!("failed to serialize read model: {e}"));
        assert_json_subset(expected, &actual, "readModel");
        self
    }

    /// Runs a custom inspection closure against `&R`.
    pub fn then_read_model_with(&self, check: impl FnOnce(&R)) -> &Self {
        check(&self.read_model);
        self
    }
}

/// Outcome of executing a [`Query`] against a [`ReadModel`] (`readModelThen` in `given-when-then/v1.yaml`).
pub struct ReadModelDecision<R: ReadModel, O, E> {
    read_model: R,
    result: Result<O, UnitFailure<E>>,
}

impl<R: ReadModel, O: fmt::Debug, E: fmt::Debug + fmt::Display> fmt::Debug
    for ReadModelDecision<R, O, E>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadModelDecision").finish_non_exhaustive()
    }
}

impl<R: ReadModel, O, E: fmt::Display> ReadModelDecision<R, O, E> {
    /// Asserts that the query returned `expected` (`then.result`).
    #[track_caller]
    pub fn then_result(&self, expected: &O) -> &Self
    where
        O: PartialEq + fmt::Debug,
    {
        match &self.result {
            Ok(actual) => assert_eq!(actual, expected, "query result mismatch"),
            Err(err) => panic!("expected query result {expected:?}, but query failed: {err}"),
        }
        self
    }

    /// Asserts that the serialized query output contains `expected` as a JSON subset (`then.result`).
    #[track_caller]
    pub fn then_result_json(&self, expected: &Value) -> &Self
    where
        O: serde::Serialize,
    {
        match &self.result {
            Ok(actual) => {
                let actual_json = serde_json::to_value(actual)
                    .unwrap_or_else(|e| panic!("failed to serialize query result: {e}"));
                assert_json_subset(expected, &actual_json, "result");
            }
            Err(err) => panic!("expected query result {expected}, but query failed: {err}"),
        }
        self
    }

    /// Asserts that the materialized read model equals `expected` (`then.readModel`).
    #[track_caller]
    pub fn then_read_model(&self, expected: &R) -> &Self
    where
        R: PartialEq + fmt::Debug,
    {
        assert_eq!(&self.read_model, expected, "read model mismatch");
        self
    }

    /// Asserts that the serialized read model contains `expected` as a JSON subset (`then.readModel`).
    #[track_caller]
    pub fn then_read_model_json(&self, expected: &Value) -> &Self
    where
        R: serde::Serialize,
    {
        let actual = serde_json::to_value(&self.read_model)
            .unwrap_or_else(|e| panic!("failed to serialize read model: {e}"));
        assert_json_subset(expected, &actual, "readModel");
        self
    }

    /// Runs a custom inspection closure against the underlying [`ReadModel`].
    pub fn then_read_model_with(&self, check: impl FnOnce(&R)) -> &Self {
        check(&self.read_model);
        self
    }

    /// Asserts that the query was rejected (e.g. unauthorized actor or query error) with a message
    /// containing `expected_substring`.
    #[track_caller]
    pub fn then_rejected_reason(&self, expected_substring: &str) -> &Self {
        match &self.result {
            Err(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains(expected_substring),
                    "expected query error containing {expected_substring:?}, got {msg:?}"
                );
            }
            Ok(_) => panic!(
                "expected query error containing {expected_substring:?}, but query succeeded"
            ),
        }
        self
    }
}
