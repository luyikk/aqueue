use async_lock::Semaphore;
use std::future::Future;

/// Used to control task parallelism queue
pub struct SemaphoreQueue {
    semaphore: Semaphore,
}

impl Default for SemaphoreQueue {
    #[inline]
    fn default() -> Self {
        SemaphoreQueue {
            semaphore: Semaphore::new(5),
        }
    }
}

impl SemaphoreQueue {
    #[inline]
    pub fn new(n: usize) -> SemaphoreQueue {
        SemaphoreQueue {
            semaphore: Semaphore::new(n),
        }
    }

    #[inline]
    pub async fn run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.semaphore.acquire().await;
        call(arg).await
    }
}
