//! What a `last:n` append costs once the topic keeps a count.
//!
//! ```text
//! cargo run --profile exp --example countbench
//! ```
//!
//! Each append is waited on, so it lands in its own gc drain. That is the
//! case the old trim was worst at: a burst let one scan serve many appends,
//! while a topic taking one append at a time paid a whole scan for each.

use std::time::{Duration, Instant};

use xs::{Frame, Fsync, ReadOptions, Store, StoreOptions, TTL};

const APPENDS: usize = 1_000;

fn options() -> StoreOptions {
    StoreOptions::builder()
        .fsync(Fsync::Never)
        .ttl_sweep(Duration::from_secs(3600))
        .build()
}

#[tokio::main]
async fn main() {
    println!(
        "{:>10}  {:>14}  {:>14}  {:>12}  {:>9}",
        "n", "entries/append", "gc us/append", "us/append", "held"
    );

    for n in [50_000usize, 500_000, 5_000_000] {
        let dir = tempfile::TempDir::new().unwrap();
        let store = Store::open(dir.path().to_path_buf(), options()).unwrap();

        // Fill to exactly n, so every append below is one in and one out.
        for _ in 0..n {
            store
                .append(Frame::builder("bench").ttl(TTL::Last(n as u32)).build())
                .unwrap();
        }
        store.wait_for_gc().await;

        let before = store.stats();
        let started = Instant::now();
        for _ in 0..APPENDS {
            store
                .append(Frame::builder("bench").ttl(TTL::Last(n as u32)).build())
                .unwrap();
            store.wait_for_gc().await;
        }
        let wall = started.elapsed();
        let after = store.stats();

        let held = store
            .read_sync(ReadOptions::builder().topic("bench".to_string()).build())
            .count();

        println!(
            "{n:>10}  {:>14.1}  {:>14.2}  {:>12.1}  {:>9}",
            (after.trim_scanned - before.trim_scanned) as f64 / APPENDS as f64,
            (after.gc_nanos - before.gc_nanos) as f64 / APPENDS as f64 / 1_000.0,
            wall.as_nanos() as f64 / APPENDS as f64 / 1_000.0,
            held,
        );
    }
}
