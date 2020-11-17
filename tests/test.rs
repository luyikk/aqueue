
#![feature(async_closure)]

use std::error::Error;
use tokio::time::{delay_for, Duration};
use std::sync::Arc;
use std::time::Instant;
use acter_queue::AQueue;

static mut VALUE:i32=0;

#[tokio::test]
async fn test()->Result<(),Box<dyn Error+Sync+Send>> {
    let queue = Arc::new(AQueue::new());

    let a_queue = queue.clone();
    tokio::spawn(async move {
        let x = a_queue.run(async move |_| {
            println!("a");
            delay_for(Duration::from_secs(1)).await;
            Ok(1)
        }, ()).await;

        println!("{:?}", x);
    });


    let a_queue = queue.clone();
    tokio::spawn(async move {
        for i in 0..100 {
            a_queue.run(async move |_| {
                println!("b:{}",i);
                Ok(())
            }, ()).await.unwrap();
        }
    });

    delay_for(Duration::from_secs(2)).await;



    let start = Instant::now();
    let mut v=0i32;
    for i in 0..2000000 {
        v= queue.run(async move |x| unsafe {
            VALUE += x;
            Ok(VALUE)
        }, i).await?;
    }

    println!("{} {}", start.elapsed().as_secs_f32(),v);

    assert_eq!(v,-1455759936);

    Ok(())
}
