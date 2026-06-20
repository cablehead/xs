// Throughput benchmarks for the store and actor hot paths.
//
// Run with: cargo bench --bench throughput
//
// Dimensions, each over N tiny frames:
//
//   append        raw store writes (the floor for everything else)
//   replay        historical read_sync scan (what `start: "first"` leans on)
//   replay-topic  topic-filtered scan: idx_topic walk + a point-read per
//                 matching frame (half the stream matches)
//   actor-state   frames through an actor with a trivial state-threading
//                 closure; eval is microseconds, so this number is per-frame
//                 framework overhead
//   actor-mixed   same actor over a stream where only 10% of frames match
//                 its topic; measures the cost of frames an actor ignores
//   actor-filtered  same mixed stream, but the actor declares topics: ["ev"]
//                 so ignored frames are filtered at the read level and never
//                 reach the actor loop
//   actor-emit    the counter, emitting one output frame per input,
//                 exercising the buffered .append + flush path
//
// Output is one parseable line per dimension:
//   <name> frames=<n> ms=<elapsed> frames_per_s=<rate> us_per_frame=<cost>
//
// frames= is always the total stream length processed, including frames a
// dimension filters out or ignores. Numbers are only comparable on the same
// hardware.

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

/// Append n frames; every `stride`-th is "ev", the rest "noise".
/// stride=1 means all "ev".
fn seed(store: &Store, n: usize, stride: usize) {
    for i in 0..n {
        let topic = if i % stride == 0 { "ev" } else { "noise" };
        store.append(Frame::builder(topic).build()).unwrap();
    }
}

fn bench_append() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    let start = Instant::now();
    seed(&store, N, 1);
    report("append", N, start.elapsed());
}

fn bench_replay() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    seed(&store, N, 1);
    let options = ReadOptions::builder().build();
    let start = Instant::now();
    let count = store.read_sync(options).count();
    assert_eq!(count, N);
    report("replay", N, start.elapsed());
}

fn bench_replay_topic() {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    seed(&store, N, 2); // half "ev", half "noise"
    let options = ReadOptions::builder().topic("ev".to_string()).build();
    let start = Instant::now();
    let count = store.read_sync(options).count();
    assert_eq!(count, N / 2);
    report("replay-topic", N, start.elapsed());
}

/// Register `closure` as an actor over a store pre-seeded with N frames
/// (every `stride`-th topic "ev"), then time from registration to the
/// actor's "bench.done" append.
async fn run_actor_bench(name: &str, closure: &str, stride: usize) {
    let temp_dir = TempDir::new().unwrap();
    let store = Store::new(temp_dir.path().to_path_buf()).unwrap();
    seed(&store, N, stride);

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

/// A counter over "ev" frames that appends "bench.done" at `target`.
/// With emit, every counted frame also appends an output frame. With
/// filtered, the actor declares topics: ["ev"] so non-matching frames are
/// filtered at the read level and never reach the actor loop.
fn actor_closure(emit: bool, filtered: bool, target: usize) -> String {
    let body = if emit {
        r#"($n | into string) | .append "bench.out""#
    } else {
        ""
    };
    let topics = if filtered { r#"  topics: ["ev"]"# } else { "" };
    format!(
        r#"{{
  run: {{|frame, state|
    if $frame.topic != "ev" {{ return {{next: $state}} }}
    let n = ($state | default 0) + 1
    {body}
    if $n == {target} {{ null | .append "bench.done" }}
    {{next: $n}}
  }}
  start: "first"
{topics}
}}"#
    )
}

fn main() {
    bench_append();
    bench_replay();
    bench_replay_topic();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        run_actor_bench("actor-state", &actor_closure(false, false, N), 1).await;
        run_actor_bench("actor-mixed", &actor_closure(false, false, N / 10), 10).await;
        run_actor_bench("actor-filtered", &actor_closure(false, true, N / 10), 10).await;
        run_actor_bench("actor-emit", &actor_closure(true, false, N), 1).await;
    });
}
