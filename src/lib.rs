pub mod actor;
pub mod queue;

pub use actor::Actor;
pub use queue::AQueue;

/// inner call wait ms throw time error
/// need on feature "wait_tokio" or "wait_async_std"
/// tokio:
/// ``` toml
/// aqueue = { version = "^1.2.6", features = ["wait_tokio"] }
/// ```
/// async_std:
/// ``` toml
/// aqueue = { version = "^1.2.6", features = ["wait_async_std"] }
/// ```
#[doc = r#"
# Example
```
    async fn test_unsafe_blocking(&self, name: String, gold: f64) -> Result<bool> {
        inner_wait!(self, 30000, |_| async move { DB.insert_user(name, gold).await }).await?
    }
```
"#]
#[macro_export]
macro_rules! inner_wait {
    ($actor:expr,$timeout:expr,$fun:expr) => {{
        #[cfg(feature = "wait_tokio")]
        {
            tokio::time::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
        }
        #[cfg(feature = "wait_async_std")]
        {
            async_std::future::timeout(std::time::Duration::from_millis($timeout), $actor.inner_call($fun))
        }
    }};
}
