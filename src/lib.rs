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

/// inner call wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # tokio runtime:
/// ``` toml
/// aqueue = { version = "^1.2.10", features = ["tokio_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         inner_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    };
}

/// inner call wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # async_std runtime:
/// ``` toml
/// aqueue = { version = "^1.2.10", features = ["async_std_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         inner_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    };
}

/// call_mut wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # tokio runtime:
/// ``` toml
/// aqueue = { version = "^1.3.2", features = ["tokio_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         call_mut_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! call_mut_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $model.call_mut($fun))
    };
}

/// call_mut wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # tokio runtime:
/// ``` toml
/// aqueue = { version = "^1.3.2", features = ["async_std_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         call_mut_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! call_mut_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $model.call_mut($fun))
    };
}

/// call wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # tokio runtime:
/// ``` toml
/// aqueue = { version = "^1.3.2", features = ["tokio_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         call_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "tokio_time", not(feature = "async_std_time")))]
#[macro_export]
macro_rules! call_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $model.call($fun))
    };
}

/// call wait ms throw time error
/// need on feature "tokio_time" or "async_std_time"
/// # tokio runtime:
/// ``` toml
/// aqueue = { version = "^1.3.2", features = ["async_std_time"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         call_wait!(self, 30000, |_inner| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "async_std_time", not(feature = "tokio_time")))]
#[macro_export]
macro_rules! call_wait {
    ($model:expr,$timeout:expr,$fun:expr) => {
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $model.call($fun))
    };
}
