use aqueue::{PCModel, SemaphoreQueue};
use std::sync::Arc;

#[tokio::test]
async fn test_base() {
    let queue = Arc::new(SemaphoreQueue::new(5));
    let mut tasks = vec![];
    let now = std::time::Instant::now();
    for i in 0..10 {
        let queue = queue.clone();
        tasks.push(tokio::spawn(async move {
            let g = queue
                .run(
                    |x| async move {
                        println!("{}: acquired", x);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        println!("{i} run end");
                        x
                    },
                    i,
                )
                .await;
            g
        }))
    }

    for task in tasks {
        let i = task.await.unwrap();
        println!("{i} finished");
    }

    assert!(now.elapsed().as_secs() >= 2 && now.elapsed().as_secs() < 3);
}

#[tokio::test]
async fn test_pc_model() {
    struct Foo;

    impl Foo {
        pub async fn run(&self, x: i32) -> i32 {
            println!("pc mode {x}: acquired");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("pc mode {x}: run end");
            x
        }
    }

    trait IFoo {
        async fn pc_run(&self, x: i32) -> i32;
    }

    impl IFoo for PCModel<Foo> {
        async fn pc_run(&self, x: i32) -> i32 {
            self.call(|inner| async move { inner.run(x).await }).await
        }
    }

    let foo = Arc::new(PCModel::new(Foo, 5));
    let mut tasks = vec![];

    let now = std::time::Instant::now();
    for i in 0..10 {
        let foo = foo.clone();
        tasks.push(tokio::spawn(async move { foo.pc_run(i).await }))
    }

    for task in tasks {
        let i = task.await.unwrap();
        println!("pc mode {i}: finished");
    }

    assert!(now.elapsed().as_secs() >= 2 && now.elapsed().as_secs() < 3);
}
