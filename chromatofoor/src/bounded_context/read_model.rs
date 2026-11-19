use crate::{bounded_context::BoundedContext, common::Kind};
use std::future::Future;

/// An ESDM `read-model` — a materialized view within a [`BoundedContext`] projected from
/// one or more domain events.
///
/// [`#[chromatofoor::read_model(scope = ..., events = [...])]`](crate::read_model)
/// generates a `<ReadModel>Events` enum and implements [`architecturekit::Projection`] by
/// dispatching each listed event `E` to [`Project<E>::project`] and forwarding
/// [`Projection::seen`](architecturekit::Projection::seen) to every view field on the struct.
///
/// # Example
///
/// ```
/// # use chromatofoor::{
/// #     architecturekit::{Envelope, InMemoryView, KeyedView, Subject, ViewError, testing},
/// #     bounded_context::Project,
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::event(scope = Ordering)]
/// pub struct OrderPlaced {
///     pub order_id: String,
///     pub total_cents: u64,
/// }
///
/// #[chromatofoor::read_model(scope = Ordering, events = [OrderPlaced])]
/// pub struct OrderTotals {
///     pub totals: InMemoryView<String, (String, u64)>,
/// }
///
/// impl Project<OrderPlaced> for OrderTotals {
///     async fn project(&self, event: Envelope<OrderPlaced>) -> Result<(), ViewError> {
///         self.totals
///             .insert(&event, (event.data.order_id.clone(), event.data.total_cents))?;
///         Ok(())
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// let read_model = OrderTotals {
///     totals: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
/// };
///
/// testing::project(
///     &read_model,
///     &testing::stored_events(
///         &Subject::new("/commerce/ordering/order/ord-1"),
///         [OrderTotalsEvents::from(OrderPlaced {
///             order_id: "ord-1".to_owned(),
///             total_cents: 2500,
///         })],
///     ),
/// )
/// .await
/// .unwrap();
///
/// assert_eq!(
///     read_model.totals.get("ord-1").await.unwrap(),
///     Some(("ord-1".to_owned(), 2500))
/// );
/// # });
/// ```
///
/// Every event listed in `events = [...]` must have a corresponding [`Project<E>`]
/// implementation:
///
/// ```compile_fail
/// # use chromatofoor::architecturekit::InMemoryView;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::event(scope = Ordering)]
/// pub struct OrderPlaced {
///     pub order_id: String,
/// }
///
/// // Error: `OrderTotals` does not implement `Project<OrderPlaced>`
/// #[chromatofoor::read_model(scope = Ordering, events = [OrderPlaced])]
/// pub struct OrderTotals {
///     pub totals: InMemoryView<String, String>,
/// }
/// ```
///
/// And `events = [...]` must name at least one event:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::read_model(scope = Ordering, events = [])]
/// pub struct EmptyReadModel;
/// ```
pub trait ReadModel:
    Kind<Scope: BoundedContext>
    + architecturekit::Projection<Event: Send + Sync + 'static>
    + Send
    + Sync
{
    /// Topic paths of the domain events projected into this read model.
    const EVENTS: &'static [&'static str] = &[];
}

/// Projects an incoming event [`architecturekit::Envelope<E>`] into a [`ReadModel`].
pub trait Project<E>: ReadModel {
    /// Applies `event` to update this read model's underlying view(s).
    fn project(
        &self,
        event: architecturekit::Envelope<E>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// String-keyed in-memory document view backed by [`architecturekit::InMemoryView`].
///
/// Provides [`Default`], [`serde::Serialize`] (serializing the latest document for snapshot/GWT
/// assertions), revision tracking via [`Self::seen`] / [`architecturekit::Revisioned`], and
/// ergonomic [`Self::upsert`] / [`Self::upsert_subject`] / [`Self::get`] / [`Self::all`] /
/// [`Self::latest`] helpers for [`ReadModel`] projections and [`Query`](crate::bounded_context::Query)
/// implementations.
#[derive(Debug)]
pub struct DocumentView<T> {
    inner: architecturekit::InMemoryView<String, (String, T)>,
}

impl<T> DocumentView<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Creates an empty [`DocumentView`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: architecturekit::InMemoryView::new(|(key, _): &(String, T)| key.clone()),
        }
    }

    /// Returns a reference to the underlying [`architecturekit::InMemoryView`].
    #[must_use]
    pub const fn inner(&self) -> &architecturekit::InMemoryView<String, (String, T)> {
        &self.inner
    }

    /// Extracts the trailing instance identifier segment from `event.subject`.
    #[must_use]
    pub fn subject_id<E>(event: &architecturekit::Envelope<E>) -> &str {
        event
            .subject
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or_default()
    }

    /// Records `event_id` as processed on the underlying [`architecturekit::InMemoryView`].
    pub fn seen(&self, event_id: architecturekit::Revision) {
        self.inner.seen(event_id);
    }

    /// Inserts a new document under `key` for `event`.
    ///
    /// # Errors
    ///
    /// Returns [`architecturekit::ViewError`] if `key` already exists with an older revision.
    pub fn insert<E>(
        &self,
        key: impl Into<String>,
        event: &architecturekit::Envelope<E>,
        value: T,
    ) -> Result<architecturekit::Outcome, architecturekit::ViewError> {
        self.inner.insert(event, (key.into(), value))
    }

    /// Inserts or updates the document under `key` for `event`.
    ///
    /// # Errors
    ///
    /// Returns [`architecturekit::ViewError`] if the underlying view rejects the key update.
    pub fn upsert<E>(
        &self,
        key: &str,
        event: &architecturekit::Envelope<E>,
        change: impl FnOnce(&mut T),
    ) -> Result<architecturekit::Outcome, architecturekit::ViewError>
    where
        T: Default,
    {
        self.inner.upsert(key, event, |(entry_key, record)| {
            if entry_key.is_empty() {
                key.clone_into(entry_key);
            }
            change(record);
        })
    }

    /// Inserts or updates the document keyed by `event.subject`'s trailing ID segment.
    ///
    /// # Errors
    ///
    /// Returns [`architecturekit::ViewError`] if the underlying view rejects the key update.
    pub fn upsert_subject<E>(
        &self,
        event: &architecturekit::Envelope<E>,
        change: impl FnOnce(&mut T),
    ) -> Result<architecturekit::Outcome, architecturekit::ViewError>
    where
        T: Default,
    {
        self.upsert(Self::subject_id(event), event, change)
    }

    /// Updates an existing document under `key` for `event`, if present.
    ///
    /// # Errors
    ///
    /// Returns [`architecturekit::ViewError`] if the underlying view rejects the update.
    pub fn update<E>(
        &self,
        key: &str,
        event: &architecturekit::Envelope<E>,
        change: impl FnOnce(&mut T),
    ) -> Result<architecturekit::Outcome, architecturekit::ViewError> {
        self.inner.update(key, event, |(_, record)| change(record))
    }

    /// Deletes the document under `key` for `event`.
    pub fn delete<E>(
        &self,
        key: &str,
        event: &architecturekit::Envelope<E>,
    ) -> architecturekit::Outcome {
        self.inner.delete(key, event)
    }

    /// Fetches a clone of the document stored under `key`, if any.
    pub async fn get(&self, key: &str) -> Option<T> {
        use architecturekit::KeyedView as _;
        let Ok(entry) = self.inner.get(key).await;
        entry.map(|(_, record)| record)
    }

    /// Returns clones of all documents in insertion order.
    pub async fn all(&self) -> Vec<T> {
        use architecturekit::View as _;
        use futures_util::TryStreamExt as _;
        let Ok(entries) = self.inner.all().try_collect::<Vec<_>>().await;
        entries.into_iter().map(|(_, record)| record).collect()
    }

    /// Returns the most recently inserted document, or `T::default()` when empty.
    pub async fn latest(&self) -> T
    where
        T: Default,
    {
        self.all().await.into_iter().next_back().unwrap_or_default()
    }

    fn snapshot_all(&self) -> Vec<T> {
        use architecturekit::View as _;
        use futures_util::Stream as _;
        use std::task::{Context, Poll, Waker};

        let mut stream = std::pin::pin!(self.inner.all());
        let mut cx = Context::from_waker(Waker::noop());
        let mut out = Vec::new();
        while let Poll::Ready(Some(Ok((_, record)))) = stream.as_mut().poll_next(&mut cx) {
            out.push(record);
        }
        out
    }
}

impl<T> Default for DocumentView<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> serde::Serialize for DocumentView<T>
where
    T: Clone + Default + serde::Serialize + Send + Sync + 'static,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let latest = self
            .snapshot_all()
            .into_iter()
            .next_back()
            .unwrap_or_default();
        latest.serialize(serializer)
    }
}

impl<T> architecturekit::Revisioned for DocumentView<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Error = std::convert::Infallible;

    fn revision(
        &self,
    ) -> impl Future<Output = Result<Option<architecturekit::Revision>, Self::Error>> + Send {
        self.inner.revision()
    }

    fn wait_for(
        &self,
        revision: architecturekit::Revision,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.inner.wait_for(revision)
    }
}
