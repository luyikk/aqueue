# fast speed thread safe async execute queue

#Examples
```rust
use acter_queue::AQueue;
static mut VALUE:i32=0;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error+Sync+Send>> {
    let queue = AQueue::new();
    let mut v=0i32;
    for i in 0..2000000 {
        v= queue.run(async move |x| unsafe {
            // thread safe execute
            VALUE += x;
            Ok(VALUE)
        }, i).await?;
    }

    assert_eq!(v,-1455759936);
   
}

```