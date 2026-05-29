//! # aqueue — Fast, Thread-Safe Async Execution Queue
//!
//! `aqueue` provides three concurrency models for protecting shared state in
//! async Rust code, each backed by a different locking primitive:
//!
//! | Type | Primitive | Best for |
//! |---|---|---|
//! | [`Actor<I>`] | Mutex | serial / write-heavy workloads |
//! | [`RwModel<I>`] | RwLock | read-heavy workloads |
//! | [`PCModel<I>`] | Semaphore | bounded parallelism / rate limiting |
//!
//! The low-level queue primitives ([`AQueue`], [`RwQueue`], [`SemaphoreQueue`])
//! are also exported for custom use cases.
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |---|---|
//! | `tokio_time` | Enables [`inner_wait!`], [`call_wait!`], [`call_mut_wait!`] with tokio timeouts |
//! | `async_std_time` | Same macros backed by async-std timeouts |

mod actor;
mod inner_store;
mod mutex;
mod pc_model;
mod rw_model;
mod rwlock;
mod semaphore;

pub use actor::Actor;
pub use mutex::AQueue;
pub use pc_model::PCModel;
pub use rw_model::RwModel;
pub use rwlock::RwQueue;
pub use semaphore::SemaphoreQueue;

/// Run [`Actor::inner_call`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `tokio_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["tokio_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = inner_wait!(actor, 5000, |inner| async move {
///     inner.get_mut().do_work()
/// }).await?;
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    };
}

/// Run [`Actor::inner_call`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `async_std_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["async_std_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = inner_wait!(actor, 5000, |inner| async move {
///     inner.get_mut().do_work()
/// }).await?;
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    };
}

/// Run [`RwModel::call_mut`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `tokio_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["tokio_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = call_mut_wait!(model, 5000, |mut inner| async move {
///     inner.mutate()
/// }).await?;
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! call_mut_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $model.call_mut($fun))
    };
}

/// Run [`RwModel::call_mut`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `async_std_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["async_std_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = call_mut_wait!(model, 5000, |mut inner| async move {
///     inner.mutate()
/// }).await?;
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! call_mut_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $model.call_mut($fun))
    };
}

/// Run [`RwModel::call`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `tokio_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["tokio_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = call_wait!(model, 5000, |inner| async move {
///     inner.read_value()
/// }).await?;
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! call_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $model.call($fun))
    };
}

/// Run [`RwModel::call`] with a millisecond timeout, returning
/// `Err` if the deadline is exceeded.
///
/// Requires the `async_std_time` feature.
///
/// ```toml
/// aqueue = { version = "^1.4", features = ["async_std_time"] }
/// ```
///
/// # Example
///
/// ```ignore
/// let result = call_wait!(model, 5000, |inner| async move {
///     inner.read_value()
/// }).await?;
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! call_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $model.call($fun))
    };
}
