use std::future::Future;

use crate::{
    bounded_context::{
        BoundedContext, Command,
        command::{CommandExecutionError, CommandScope, verify_zero_event_preconditions},
    },
    common::{Identified, Kind},
};

/// An ESDM `aggregate` — a persistent, container-based consistency unit within a
/// [`BoundedContext`].
///
/// # Identification Strategies (`identified_by`)
///
/// ## 1. `generated(uuid)` *(default)*
///
/// Creation commands (`#[lifecycle(create)]`) mint a fresh UUID subject; mutation commands
/// address an existing aggregate via `id: Identifier`:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified, Identifier}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering)]
/// #[derive(Clone, Default)]
/// pub struct Order;
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
///
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     PlaceOrder,
///     CancelOrder { id: Identifier },
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(Order::IDENTIFIED_BY, &[IdSource::Generated("uuid")]);
/// assert!((PlaceOrder {}).subject().as_str().starts_with("/commerce/ordering/order/"));
/// assert_eq!(
///     (CancelOrder { id: "550e8400-e29b-41d4-a716-446655440000".into() }).subject().as_str(),
///     "/commerce/ordering/order/550e8400-e29b-41d4-a716-446655440000"
/// );
/// ```
///
/// ## 2. `state(<field>)`
///
/// Identified by an [`Identifier`](crate::common::Identifier) field on the aggregate's state struct:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified, Identifier}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// #[derive(Clone, Default)]
/// pub struct Order {
///     pub order_id: Option<Identifier>,
/// }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
///
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     PlaceOrder { order_id: Identifier },
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(Order::IDENTIFIED_BY, &[IdSource::State("order_id")]);
/// assert_eq!(
///     (PlaceOrder { order_id: "ord-1".into() }).subject().as_str(),
///     "/commerce/ordering/order/ord-1"
/// );
/// ```
///
/// Referencing a nonexistent field in `identified_by = state(...)` is rejected at
/// compile time:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = state(missing_field))]
/// #[derive(Clone, Default)]
/// pub struct Order {
///     pub order_id: Option<Identifier>,
/// }
/// ```
///
/// ## 3. `static("<value>")`
///
/// Singleton aggregate within the bounded context addressed at a fixed subject suffix:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = static("singleton"))]
/// #[derive(Clone, Default)]
/// pub struct StorefrontSettings;
/// # #[chromatofoor::event(scope = StorefrontSettings)]
/// # pub enum StorefrontSettingsEvent { MaintenanceEnabled }
/// # impl chromatofoor::bounded_context::Event<StorefrontSettings> for MaintenanceEnabled {
/// #     fn evolve(&self, state: StorefrontSettings) -> StorefrontSettings { state }
/// # }
///
/// #[chromatofoor::command(scope = StorefrontSettings)]
/// pub enum StorefrontSettingsCommand {
///     EnableMaintenanceMode,
/// }
/// # impl chromatofoor::bounded_context::Command<StorefrontSettings> for EnableMaintenanceMode {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &StorefrontSettings) -> Result<Vec<StorefrontSettingsEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(StorefrontSettings::IDENTIFIED_BY, &[IdSource::Static("singleton")]);
/// assert_eq!(
///     (EnableMaintenanceMode {}).subject().as_str(),
///     "/commerce/ordering/storefront-settings/singleton"
/// );
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Aggregate`",
    label = "expected an `Aggregate` here",
    note = "aggregate-scoped event enums (`#[event(scope = ...)] pub enum ...`) require an `Aggregate` scope; for freestanding bounded-context events, use `#[event(scope = <BoundedContext>)] pub struct ...`"
)]
pub trait Aggregate:
    Kind<Scope: BoundedContext> + Identified + CommandScope + architecturekit::State
{
    /// Executes `command` against `store` via [`architecturekit::Store::execute`], additionally
    /// verifying any explicit database preconditions (such as `isSubjectPopulated` on mutation
    /// commands) when [`Command::decide`] emits zero events.
    fn execute<C>(
        store: &architecturekit::Store,
        command: C,
    ) -> impl Future<
        Output = Result<
            Vec<architecturekit::eventsourcingdb::Event>,
            CommandExecutionError<C::Error>,
        >,
    > + Send
    where
        Self: Send + Sync + 'static,
        Self::Event: Send + Sync + 'static,
        C: Command<Self> + Send + Sync + 'static,
        C::Error: Send + 'static,
    {
        async move {
            let preconditions = architecturekit::Command::preconditions(&command);
            let events = store.execute(&C::decider(), command).await?;
            if events.is_empty() {
                verify_zero_event_preconditions(store, &preconditions).await?;
            }
            Ok(events)
        }
    }
}

// TODO: Move shared test fixtures (commands, events, aggregates, etc.) into a dedicated
// `testing/fixtures` module so they can be reused across unit tests and doctests.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]
mod tests {
    use std::sync::Arc;

    use crate::{
        self as chromatofoor,
        architecturekit::{InMemoryEventStore, Store},
        bounded_context::{Aggregate, Command, CommandExecutionError, Event, PreconditionFailure},
        common::Identifier,
        domain::DomainBuilder,
        serde_json::json,
    };

    #[chromatofoor::domain]
    pub struct Banking;

    #[chromatofoor::bounded_context(scope = Banking)]
    pub struct Finance;

    #[chromatofoor::actor(scope = Finance, type = human)]
    pub struct Customer;

    #[chromatofoor::aggregate(scope = Finance)]
    #[derive(Clone, Default)]
    pub enum Account {
        #[default]
        None,
        Created {
            balance: i64,
        },
    }

    #[chromatofoor::event(scope = Account)]
    pub enum AccountEvent {
        AccountOpened { balance: i64 },
        FundsDeposited { amount: i64, new_balance: i64 },
    }

    impl Event<Account> for AccountOpened {
        fn evolve(&self, _state: Account) -> Account {
            Account::Created {
                balance: self.balance,
            }
        }
    }

    impl Event<Account> for FundsDeposited {
        fn evolve(&self, _state: Account) -> Account {
            Account::Created {
                balance: self.new_balance,
            }
        }
    }

    #[chromatofoor::command(scope = Account, actors = [Customer])]
    pub enum AccountCommand {
        #[lifecycle(create)]
        OpenAccount {
            initial_balance: i64,
        },
        DepositFunds {
            id: Identifier,
            amount: i64,
        },
    }

    impl Command<Account> for OpenAccount {
        type Error = &'static str;

        async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
            Ok(vec![
                AccountOpened {
                    balance: self.initial_balance,
                }
                .into(),
            ])
        }
    }

    impl Command<Account> for DepositFunds {
        type Error = &'static str;

        async fn decide(self, state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
            let Account::Created { balance } = state else {
                return Ok(vec![]);
            };
            if self.amount == 0 {
                return Ok(vec![]);
            }
            Ok(vec![
                FundsDeposited {
                    amount: self.amount,
                    new_balance: balance + self.amount,
                }
                .into(),
            ])
        }
    }

    #[tokio::test]
    async fn zero_event_mutation_fails_on_missing_aggregate_and_succeeds_when_populated() {
        let store =
            Arc::new(Store::builder(InMemoryEventStore::new(), "https://bank.test").build());
        let missing_id: Identifier = "00000000-0000-0000-0000-000000000000".into();

        // 1. Direct Aggregate::execute rejects 0-event mutation against an uncreated stream:
        let err = Account::execute(
            &store,
            DepositFunds {
                id: missing_id.clone(),
                amount: 50,
            },
        )
        .await
        .expect_err("mutating uncreated aggregate must fail even when decide returns Ok(vec![])");

        assert!(matches!(
            err,
            CommandExecutionError::Precondition(PreconditionFailure::SubjectNotPopulated { ref subject })
                if subject == "/banking/finance/account/00000000-0000-0000-0000-000000000000"
        ));

        // 2. DomainRuntime::dispatch_command also surfaces the precondition failure (used by TUI/HTTP/MCP):
        let runtime = DomainBuilder::<Banking>::new(Arc::clone(&store))
            .register_command::<OpenAccount, _>()
            .register_command::<DepositFunds, _>()
            .build()
            .await
            .unwrap();

        let dispatch_err = runtime
            .dispatch_command(
                "finance",
                "account",
                "deposit-funds",
                json!({ "id": missing_id, "amount": 50 }),
                Some("customer"),
            )
            .await
            .expect_err("runtime dispatch must fail for missing aggregate");
        assert!(dispatch_err.to_string().contains("isSubjectPopulated"));

        // 3. Create an account, then verify an idempotent $0 deposit (0 events) on that existing ID succeeds:
        let created = runtime
            .dispatch_command(
                "finance",
                "account",
                "open-account",
                json!({ "initial_balance": 100 }),
                Some("customer"),
            )
            .await
            .unwrap();
        let created_id = created[0].subject().rsplit('/').next().unwrap().to_owned();

        let noop_events = runtime
            .dispatch_command(
                "finance",
                "account",
                "deposit-funds",
                json!({ "id": created_id, "amount": 0 }),
                Some("customer"),
            )
            .await
            .expect("idempotent 0-event mutation on existing aggregate must succeed");
        assert!(noop_events.is_empty());
    }
}
