// Throughput benchmarks for the store and actor hot paths.
//
// Run with: cargo bench --bench throughput
//
// Four dimensions, each over the same N tiny frames:
//
//   append       raw store writes (the floor for everything else)
//   replay       historical read_sync scan (what `start: "first"` leans on)
//   actor-state  frames through an actor with a trivial state-threading
//                closure; eval is microseconds, so this number is per-frame
//                framework overhead
//   actor-emit   same counter, but emitting one output frame per input,
//                exercising the buffered .append + flush path
//
// Output is one parseable line per dimension:
//   <name> frames=<n> ms=<elapsed> frames_per_s=<rate> us_per_frame=<cost>
//
// Numbers are only comparable on the same hardware.

use std::time::{Duration, Instant};

use tempfile::TempDir;

use xs::store::{Frame, ReadOptions, Store};

const N: usize = 100_000;

fn report(name: &str, n: usize, elapsed: Duration) {
    let ms = elapsed.as_secs_f64() * 1e3;
    let rate = n as f64 / elapsed.as_secs_f64();
    let us = elapsed.as_secs_f64() * 1e6 / n as f64;
    println!("{name} frames={n} ms={ms:.0} frames_per_s={rate:.0} us_per_frame={us:.2}");
}

fn seed(store: &Store, n: usize) {
    for _ in 0..n {
        store.append(Frame::builder("ev").build()).unwrap();
    }
}

fn bench_append() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    let start = Instant::now();
    seed(&store, N);
    report("append", N, start.elapsed());
}

fn bench_replay() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    seed(&store, N);
    let options = ReadOptions::builder().build();
    let start = Instant::now();
    let count = store.read_sync(options).count();
    assert_eq!(count, N);
    report("replay", N, start.elapsed());
}

/// Register `closure` as an actor over a store pre-seeded with N "ev" frames,
/// then time from registration to the actor's "bench.done" append.
async fn run_actor_bench(name: &str, closure: &str) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    seed(&store, N);

    {
        let store = store.clone();
        drop(tokio::spawn(async move {
            xs::processor::actor::run(store).await.unwrap();
        }));
    }

    let start = Instant::now();
    store
        .append(
            Frame::builder("xs.actor.bench.create")
                .hash(store.cas_insert_sync(closure).unwrap())
                .build(),
        )
        .unwrap();

    let done = ReadOptions::builder()
        .topic("bench.done".to_string())
        .last(1usize)
        .build();
    let fin = ReadOptions::builder()
        .topic("xs.actor.bench.fin.*".to_string())
        .last(1usize)
        .build();
    loop {
        if store.read_sync(done.clone()).next().is_some() {
            break;
        }
        if let Some(frame) = store.read_sync(fin.clone()).next() {
            panic!("{name}: actor terminated without finishing: {frame:?}");
        }
        if start.elapsed() > Duration::from_secs(300) {
            panic!("{name}: timed out waiting for bench.done");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    report(name, N, start.elapsed());
}

fn actor_closure(emit: bool) -> String {
    let body = if emit {
        r#"($n | into string) | .append "bench.out""#
    } else {
        ""
    };
    format!(
        r#"{{
  run: {{|frame, state|
    if $frame.topic != "ev" {{ return {{next: $state}} }}
    let n = ($state | default 0) + 1
    {body}
    if $n == {N} {{ null | .append "bench.done" }}
    {{next: $n}}
  }}
  start: "first"
}}"#
    )
}

fn main() {
    bench_append();
    bench_replay();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_actor_bench("actor-state", &actor_closure(false)).await;
        run_actor_bench("actor-emit", &actor_closure(true)).await;
    });
}
