pub mod actor;
pub mod queue;

pub use actor::Actor;
pub use queue::AQueue;

/// inner call wait ms throw time error
/// need on feature "tokio" or "async_std"
/// # tokio:
/// ``` toml
/// aqueue = { version = "^1.2.6", features = ["tokio"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         inner_wait!(self, 30000, |_| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "tokio", not(feature = "async_std")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {{
        tokio::time::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    }};
}

/// inner call wait ms throw time error
/// need on feature "tokio" or "async_std"
/// # async_std:
/// ``` toml
/// aqueue = { version = "^1.2.6", features = ["async_std"] }
/// ```
/// # Example
/// ``` ignore
///     async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
///         inner_wait!(self, 30000, |_| async move { DB.insert_user(name, gold).await }).await?
///     }
/// ```
#[cfg(all(feature = "async_std", not(feature = "tokio")))]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {{
        async_std::future::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
    }};
}
