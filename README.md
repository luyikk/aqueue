# aqueue — Fast, Thread-Safe Async Execution Queue

[![Latest Version](https://img.shields.io/crates/v/aqueue.svg)](https://crates.io/crates/aqueue)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/aqueue)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/luyikk/aqueue)](https://rust-reportcard.xuri.me/report/github.com/luyikk/aqueue)
[![Rust CI](https://github.com/luyikk/aqueue/actions/workflows/rust.yml/badge.svg)](https://github.com/luyikk/aqueue/actions/workflows/rust.yml)

`aqueue` provides three concurrency models for protecting shared state in async Rust,
each backed by a different locking primitive — pick the one that best fits your workload:
| Type | Primitive | Best for |
|------|-----------|----------|
| [`Actor<I>`](#actori) | `async_lock::Mutex` | Serial / write-heavy / stateful workloads |
| [`RwModel<I>`](#rwmodeli) | `async_lock::RwLock` | Read-heavy workloads |
| [`PCModel<I>`](#pcmodeli) | `async_lock::Semaphore` | Bounded parallelism / rate-limiting |
The low-level queue primitives (`AQueue`, `RwQueue`, `SemaphoreQueue`) are also exported for custom use cases.

---

## Table of Contents
- [Installation](#installation)
- [Feature Flags](#feature-flags)
- [Core Types](#core-types)
  - [Actor\<I\>](#actori)
  - [RwModel\<I\>](#rwmodeli)
  - [PCModel\<I\>](#pcmodeli)
  - [Low-Level Queues](#low-level-queues)
- [Timeout Macros](#timeout-macros)
- [Examples](#examples)
  - [Actor — Basic Counter](#actor--basic-counter)
  - [Actor — SQLite Database](#actor--sqlite-database)
  - [RwModel — Read-Heavy Counter](#rwmodel--read-heavy-counter)
  - [PCModel — Concurrency Limiter](#pcmodel--concurrency-limiter)
- [Benchmarks](#benchmarks)
- [Architecture](#architecture)
- [Changelog](#changelog)
---
## Installation
```toml
[dependencies]
aqueue = "1.4"
# Optional: enable timeout macro support (tokio runtime)
# aqueue = { version = "1.4", features = ["tokio_time"] }
```
---
## Feature Flags
| Flag | Description |
|------|-------------|
| `tokio_time` | Enables `inner_wait!`, `call_wait!`, `call_mut_wait!` backed by **tokio** timeouts |
| `async_std_time` | Same three macros backed by **async-std** timeouts |
Neither flag is enabled by default. The core library only depends on [`async-lock`](https://crates.io/crates/async-lock).
---
## Core Types
### `Actor<I>`
> **Best for:** write-heavy, exclusive-access, or stateful workloads.
`Actor<I>` wraps any value `I` behind an async Mutex queue (`AQueue`). All operations execute **serially** — only one closure runs at a time — without any `Mutex<I>` in user code.
The closure parameter is `&'a InnerStore<I>` whose lifetime `'a` is tied to `&'a self`, guaranteeing the reference stays valid across every `.await` point inside the closure.
**Public API:**
| Method | Description |
|--------|-------------|
| `Actor::new(x)` | Wrap `x` in a new `Actor` |
| `async fn inner_call(f)` | Run `f` exclusively under the queue lock (FIFO) |
| `unsafe fn deref_inner()` | Borrow the inner value directly, bypassing the lock |
---
### `RwModel<I>`
> **Best for:** read-heavy workloads with occasional exclusive mutations.
`RwModel<I>` is backed by an async RwLock queue (`RwQueue`):
- `call` acquires a **shared read lock** — multiple readers run concurrently.
- `call_mut` acquires an **exclusive write lock** — no other reads or writes run concurrently.
Synchronous variants `sync_call` / `sync_mut_call` are available for non-async call sites (they yield the thread via `thread::yield_now` until the lock is free).
**Public API:**
| Method | Description |
|--------|-------------|
| `RwModel::new(x)` | Wrap `x` in a new `RwModel` |
| `async fn call(f)` | Shared read lock — closure receives `RefInner<'a, I>` |
| `async fn call_mut(f)` | Exclusive write lock — closure receives `RefMutInner<'a, I>` |
| `fn sync_call(f)` | Synchronous read (yields thread until lock is free) |
| `fn sync_mut_call(f)` | Synchronous write (yields thread until lock is free) |
---
### `PCModel<I>`
> **Best for:** capping the number of simultaneously executing tasks (e.g. HTTP connections, DB query concurrency).
`PCModel<I>` uses an async Semaphore (`SemaphoreQueue`) to enforce a maximum concurrency of `n`. Callers that exceed the limit suspend asynchronously until a permit becomes available.
> **Warning:** `PCModel::inner()` returns a direct reference to the inner value **without** acquiring any semaphore permit. Calls made through that reference are **not** subject to the configured concurrency limit, silently defeating the purpose of `PCModel`. Only use `inner()` for metadata that is safe to read without counting against the parallelism budget (e.g. configuration flags, addresses). Use `call()` for everything else.
**Public API:**
| Method | Description |
|--------|-------------|
| `PCModel::new(inner, n)` | Create a model allowing at most `n` concurrent executions |
| `fn inner()` | Direct reference — **does not** acquire a semaphore permit |
| `async fn call(f)` | Acquire one permit, run `f`, release permit on completion |
---
### Low-Level Queues
The following primitives are publicly exported for advanced use cases:
| Type | Primitive | Use case |
|------|-----------|----------|
| `AQueue` | `async_lock::Mutex` | Serial async execution |
| `RwQueue` | `async_lock::RwLock` | Concurrent reads / exclusive writes |
| `SemaphoreQueue` | `async_lock::Semaphore` | Bounded concurrency |
---
## Timeout Macros
Enable `tokio_time` (or `async_std_time`) to get three timeout macros that wrap a call with a millisecond deadline, returning `Err` on expiry:
```toml
aqueue = { version = "1.4", features = ["tokio_time"] }
```
| Macro | Wraps |
|-------|-------|
| `inner_wait!(actor, ms, f)` | `actor.inner_call(f)` |
| `call_mut_wait!(model, ms, f)` | `model.call_mut(f)` |
| `call_wait!(model, ms, f)` | `model.call(f)` |
```rust
use aqueue::{inner_wait, Actor};
// Returns Err if the operation takes longer than 5 000 ms
let result = inner_wait!(my_actor, 5000, |inner| async move {
    inner.get_mut().do_work()
}).await?;
```
---
## Examples
### Actor — Basic Counter
Four concurrent tasks accumulate into the same counter with zero data races:
```rust
use aqueue::Actor;
use std::sync::Arc;
use tokio::try_join;
#[derive(Default)]
struct Foo { count: u64, i: i128 }
impl Foo {
    fn add(&mut self, x: i32) -> i128 { self.count += 1; self.i += x as i128; self.i }
    fn get(&self) -> i128 { self.i }
    fn get_count(&self) -> u64 { self.count }
}
trait FooRunner {
    async fn add(&self, x: i32) -> i128;
    async fn get(&self) -> i128;
    async fn get_count(&self) -> u64;
}
impl FooRunner for Actor<Foo> {
    async fn add(&self, x: i32) -> i128 {
        self.inner_call(|inner| async move { inner.get_mut().add(x) }).await
    }
    async fn get(&self) -> i128 {
        self.inner_call(|inner| async move { inner.get().get() }).await
    }
    async fn get_count(&self) -> u64 {
        self.inner_call(|inner| async move { inner.get().get_count() }).await
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tf = Arc::new(Actor::new(Foo::default()));
    let (a, b, c, d) = (tf.clone(), tf.clone(), tf.clone(), tf.clone());
    let t1 = tokio::spawn(async move { for i in 0..25_000_000        { a.add(i).await; } });
    let t2 = tokio::spawn(async move { for i in 25_000_000..50_000_000  { b.add(i).await; } });
    let t3 = tokio::spawn(async move { for i in 50_000_000..75_000_000  { c.add(i).await; } });
    let t4 = tokio::spawn(async move { for i in 75_000_000..100_000_000 { d.add(i).await; } });
    try_join!(t1, t2, t3, t4)?;
    // count=100000000  value=4999999950000000
    println!("count={} value={}", tf.get_count().await, tf.get().await);
    Ok(())
}
```
---
### Actor — SQLite Database
Wrapping a SQLite connection pool in an `Actor` serialises all writes so the auto-incrementing ID is always unique, even across 100 concurrent tasks:
```rust
use anyhow::Result;
use aqueue::Actor;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
pub struct DataBases { auto_id: u32, pool: SqlitePool }
unsafe impl Send for DataBases {}
unsafe impl Sync for DataBases {}
impl DataBases {
    pub fn new(max: u32) -> Result<Actor<DataBases>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max)
            .connect_lazy("sqlite://:memory:")?;
        Ok(Actor::new(DataBases { auto_id: 0, pool }))
    }
    async fn create_table(&self) -> Result<()> {
        sqlx::query(
            r#"CREATE TABLE "user"("id" integer NOT NULL PRIMARY KEY,"name" text,"gold" real);"#,
        ).execute(&self.pool).await?;
        Ok(())
    }
    async fn insert_user(&mut self, name: &str, gold: f64) -> Result<bool> {
        self.auto_id += 1;
        let rows = sqlx::query(r#"INSERT INTO `user`(`id`,`name`,`gold`) VALUES(?,?,?)"#)
            .bind(self.auto_id).bind(name).bind(gold)
            .execute(&self.pool).await?.rows_affected();
        Ok(rows == 1)
    }
}
trait IDatabase {
    async fn create_table(&self) -> Result<()>;
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool>;
}
impl IDatabase for Actor<DataBases> {
    async fn create_table(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get().create_table().await }).await
    }
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool> {
        self.inner_call(|inner| async move {
            inner.get_mut().insert_user(&name, gold).await
        }).await
    }
}
lazy_static::lazy_static! {
    static ref DB: Actor<DataBases> = DataBases::new(50).expect("db init failed");
}
#[tokio::main]
async fn main() -> Result<()> {
    DB.create_table().await?;
    // 100 tasks * 1 000 inserts = 100 000 rows; auto_id is always unique
    let handles: Vec<_> = (0..100i64).map(|i| {
        tokio::spawn(async move {
            for j in 0..1000i64 {
                DB.insert_user(i.to_string(), j as f64).await?;
            }
            anyhow::Ok(())
        })
    }).collect();
    for h in handles { h.await??; }
    Ok(())
}
```
---
### RwModel — Read-Heavy Counter
`call` permits multiple simultaneous readers; `call_mut` takes an exclusive write lock:
```rust
use aqueue::RwModel;
use std::sync::Arc;
use tokio::try_join;
#[derive(Default)]
struct Foo { count: u64, i: i128 }
impl Foo {
    fn add(&mut self, x: i32) -> i128 { self.count += 1; self.i += x as i128; self.i }
    fn get(&self) -> i128 { self.i }
    fn get_count(&self) -> u64 { self.count }
}
trait FooRunner {
    async fn add(&self, x: i32) -> i128;
    async fn get(&self) -> i128;
    async fn get_count(&self) -> u64;
}
impl FooRunner for RwModel<Foo> {
    async fn add(&self, x: i32) -> i128 {
        self.call_mut(|mut inner| async move { inner.add(x) }).await  // exclusive write
    }
    async fn get(&self) -> i128 {
        self.call(|inner| async move { inner.get() }).await            // shared read
    }
    async fn get_count(&self) -> u64 {
        self.call(|inner| async move { inner.get_count() }).await
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tf = Arc::new(RwModel::new(Foo::default()));
    let (a, b, c, d) = (tf.clone(), tf.clone(), tf.clone(), tf.clone());
    let t1 = tokio::spawn(async move { for i in 0..25_000_000        { a.add(i).await; } });
    let t2 = tokio::spawn(async move { for i in 25_000_000..50_000_000  { b.add(i).await; } });
    let t3 = tokio::spawn(async move { for i in 50_000_000..75_000_000  { c.add(i).await; } });
    let t4 = tokio::spawn(async move { for i in 75_000_000..100_000_000 { d.add(i).await; } });
    try_join!(t1, t2, t3, t4)?;
    println!("count={} value={}", tf.get_count().await, tf.get().await);
    Ok(())
}
```
```
test rw a count:100000000 value:4999999950000000 time:5.18s  qps:19,308,000
test rw b count:100000000 value:4999999950000000 time:5.29s  qps:18,892,000
```
---
### PCModel — Concurrency Limiter
At most 4 requests are in-flight at any time; the rest queue up automatically:
```rust
use aqueue::PCModel;
use std::sync::Arc;
struct HttpClient;
impl HttpClient {
    async fn get(&self, url: &str) -> Vec<u8> { vec![] }
}
trait IClient {
    async fn fetch(&self, url: String) -> Vec<u8>;
}
impl IClient for PCModel<HttpClient> {
    async fn fetch(&self, url: String) -> Vec<u8> {
        self.call(|c| async move { c.get(&url).await }).await
    }
}
#[tokio::main]
async fn main() {
    // Allow at most 4 concurrent requests
    let client = Arc::new(PCModel::new(HttpClient, 4));
    let handles: Vec<_> = (0..20usize).map(|i| {
        let c = client.clone();
        tokio::spawn(async move {
            c.fetch(format!("https://example.com/file/{i}")).await
        })
    }).collect();
    for h in handles { h.await.unwrap(); }
}
```
---
## Benchmarks
Run the built-in [Criterion](https://github.com/bheisler/criterion.rs) benchmarks:
```shell
cargo bench
```
Four scenarios are measured (100 000 iterations each):
| Benchmark ID | Description |
|---|---|
| `single_task_actor_call` | Serial `Actor::inner_call` on a single-thread runtime |
| `multi_task_actor_call` | Concurrent `Actor::inner_call` from 2 tasks (multi-thread runtime) |
| `single_task_model_call` | Serial `RwModel::call_mut` on a single-thread runtime |
| `multi_task_model_call` | Concurrent `RwModel::call_mut` from 2 tasks (multi-thread runtime) |
---
## Architecture
```
aqueue
├── Actor<I>          ──► AQueue         (async_lock::Mutex)      exclusive, serial
├── RwModel<I>        ──► RwQueue        (async_lock::RwLock)     concurrent reads / exclusive writes
├── PCModel<I>        ──► SemaphoreQueue (async_lock::Semaphore)  bounded parallelism
│
├── InnerStore<T>     raw value store (UnsafeCell<T>; safety enforced by the queue above)
├── RefInner<'a,T>    shared-reference smart pointer  (impl Deref)
└── RefMutInner<'a,T> mutable-reference smart pointer (impl Deref + DerefMut)
```
All models store their data in `InnerStore<T>` — an `UnsafeCell<T>` with manual `Send + Sync` impls.
Thread safety is enforced entirely by the surrounding async lock primitive, keeping the lock and the
value completely separate with zero additional wrapper overhead.
---
## Changelog
See [CHANGELOG.md](CHANGELOG.md) for the full release history.
---
## License
Licensed under either of
- Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)
at your option.
