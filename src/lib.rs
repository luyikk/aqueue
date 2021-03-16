pub mod actor;
pub mod queue;

use crate::AError::*;
pub use actor::Actor;
pub use aqueue_trait::async_trait as aqueue_trait;
pub use queue::{AQueue, AQueueItem, QueueItem};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum AError {
    StrErr(String),
    Other(Box<dyn Error + Send + Sync + 'static>),
}

impl From<String> for AError {
    fn from(msg: String) -> Self {
        StrErr(msg)
    }
}

impl From<&str> for AError {
    fn from(msg: &str) -> Self {
        StrErr(msg.to_string())
    }
}

impl Display for AError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StrErr(ref msg) => {
                write!(f, "{}", msg)
            }
            Other(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

impl Error for AError {}
pub type AResult<T> = Result<T, AError>;
