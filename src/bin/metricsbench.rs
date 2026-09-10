//! Reads fjall's cache and filter counters under the access patterns xs
//! actually has, so `cache_size`, the filter policies and the pinning
//! policies can be set from numbers instead of reasoning.
//!
//! ```text
//! cargo run --profile exp --bin metricsbench -- <frames> <topics>
//! ```
//!
//! Counters are cumulative and fjall cannot reset them, so every phase is
//! reported as a delta against the phase before it.

use std::time::{Duration, Instant};

use scru128::Scru128Id;
use xs::store::KeyspaceMetrics;
use xs::{Frame, Fsync, ReadOptions, Store, StoreOptions};

fn options() -> StoreOptions {
    StoreOptions::builder()
        .fsync(Fsync::Never)
        // Park the sweeper: this measures reads, not gc.
        .ttl_sweep(Duration::from_secs(3600))
        .build()
}

/// A phase's cost: what it asked of each keyspace, over and above the phase
/// before it.
fn delta(before: &[KeyspaceMetrics], after: &[KeyspaceMetrics]) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (b, a) in before.iter().zip(after.iter()) {
        let loads = a.block_loads - b.block_loads;
        let io = a.block_load_io - b.block_load_io;
        if loads == 0 {
            continue;
        }
        let hits = loads - io;
        let queries = a.filter_queries - b.filter_queries;
        let skipped = a.io_skipped_by_filter - b.io_skipped_by_filter;
        rows.push((
            a.keyspace.to_string(),
            format!(
                "loads {loads:>9}  io {io:>9}  hit {:>6.1}%  \
                 data_io {:>8}  index_io {:>7}  filter_io {:>7}  \
                 filter_q {queries:>8}  skipped {skipped:>8}",
                100.0 * hits as f64 / loads as f64,
                a.data_block_io - b.data_block_io,
                a.index_block_io - b.index_block_io,
                a.filter_block_io - b.filter_block_io,
            ),
        ));
    }
    rows
}

fn phase(
    name: &str,
    before: &[KeyspaceMetrics],
    store: &Store,
    started: Instant,
) -> Vec<KeyspaceMetrics> {
    let after = store.metrics();
    println!("\n{name}  ({:.2}s)", started.elapsed().as_secs_f64());
    for (ks, row) in delta(before, &after) {
        println!("  {ks:<11}{row}");
    }
    after
}

fn main() {
    let mut args = std::env::args().skip(1);
    let frames: usize = args.next().map_or(500_000, |a| a.parse().unwrap());
    let topics: usize = args.next().map_or(64, |a| a.parse().unwrap());

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();

    // Fill. Frames carry a little meta so they are the size a real one is:
    // large payloads live in the CAS, not here.
    let store = Store::open(path.clone(), options()).unwrap();
    let started = Instant::now();
    let mut ids: Vec<Scru128Id> = Vec::with_capacity(frames);
    for i in 0..frames {
        let topic = format!("topic.{}", i % topics);
        let frame = store
            .append(
                Frame::builder(topic)
                    .meta(serde_json::json!({
                        "seq": i,
                        "note": "a representative amount of frame metadata",
                    }))
                    .build(),
            )
            .unwrap();
        ids.push(frame.id);
    }
    println!(
        "filled {frames} frames over {topics} topics in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    store.flush().unwrap();
    drop(store);

    // Every run below reopens, so each starts with an empty block cache.
    let run = |label: &str, body: &dyn Fn(&Store, &[Scru128Id])| {
        let store = Store::open(path.clone(), options()).unwrap();
        let base = store.metrics();
        let started = Instant::now();
        body(&store, &ids);
        phase(label, &base, &store, started);
        let m = store.metrics();
        for k in &m {
            println!(
                "  {:<11}cumulative: block {:>5.1}%  data {:>5.1}%  index {:>5.1}%  \
                 filter {:>5.1}%  file {:>5.1}%  filter_eff {:>5.1}%",
                k.keyspace,
                100.0 * k.rates.block_cache_hit.unwrap_or(0.0),
                100.0 * k.rates.data_block_cache_hit.unwrap_or(0.0),
                100.0 * k.rates.index_block_cache_hit.unwrap_or(0.0),
                100.0 * k.rates.filter_block_cache_hit.unwrap_or(0.0),
                100.0 * k.rates.table_file_cache_hit.unwrap_or(0.0),
                100.0 * k.rates.filter_efficiency.unwrap_or(0.0),
            );
        }
    };

    // 1. A full stream scan, cold.
    run("full cat, cold", &|store, _| {
        let n = store.read_sync(ReadOptions::default()).count();
        println!("  returned {n}");
    });

    // 2. The same scan twice: does the second one hit the cache at all?
    run("full cat, twice", &|store, _| {
        store.read_sync(ReadOptions::default()).count();
        let n = store.read_sync(ReadOptions::default()).count();
        println!("  returned {n} (second pass)");
    });

    // 3. Topic reads on a cold cache: the idx_topic scan plus a point read
    //    into stream per surviving id.
    run("topic reads, cold", &|store, _| {
        let mut n = 0;
        for t in 0..8 {
            let opts = ReadOptions::builder().topic(format!("topic.{t}")).build();
            n += store.read_sync(opts).count();
        }
        println!("  returned {n}");
    });

    // 4. The same topic reads, after a full scan has been through the cache.
    //    The gap against phase 3 is what a scan costs everyone else.
    run("topic reads, after a full cat", &|store, _| {
        store.read_sync(ReadOptions::default()).count();
        let mut n = 0;
        for t in 0..8 {
            let opts = ReadOptions::builder().topic(format!("topic.{t}")).build();
            n += store.read_sync(opts).count();
        }
        println!("  returned {n} (topic reads only counted in the delta above)");
    });

    // 5. Point reads by id, scattered, cold. The pattern the hash ratio and
    //    the filters exist for.
    run("point reads, scattered", &|store, ids| {
        let mut hits = 0;
        let mut i = 0usize;
        while i < ids.len() {
            if store.get(&ids[i]).is_some() {
                hits += 1;
            }
            i += 997; // co-prime-ish stride, so no locality
        }
        println!("  {hits} hits");
    });

    // 6. Point reads for ids that are not there. The only case a filter
    //    can pay for itself: a hit costs it nothing, a miss saves the read.
    run("point reads, all misses", &|store, ids| {
        let mut hits = 0;
        for i in 0..500 {
            // Same timestamp range as the real ids, so the id is plausible
            // and the range check cannot rule the table out on its own.
            let near = ids[i * 7 % ids.len()];
            let (ts, _, _, _) = (near.timestamp(), 0u32, 0u32, 0u32);
            let miss = Scru128Id::try_from_fields(ts, 0xff_ffff, 0xff_ffff, 0xffff_ffff).unwrap();
            if store.get(&miss).is_some() {
                hits += 1;
            }
        }
        println!("  {hits} hits (expected 0)");
    });

    // 7. Tail reads, the shape a follow starts with.
    run("tail, last 100", &|store, _| {
        for _ in 0..100 {
            let opts = ReadOptions::builder().last(100usize).build();
            store.read_sync(opts).count();
        }
    });
}
