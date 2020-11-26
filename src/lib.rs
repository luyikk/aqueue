
pub mod queue;
pub mod actor;

pub use queue::{AQueueItem,AQueue,QueueItem};
pub use actor::Actor;
pub use aqueue_trait::async_trait as aqueue_trait;
use std::error::Error;

pub type AError =Box<dyn Error+Send+Sync>;
pub type AResult<T>=Result<T,AError>;
