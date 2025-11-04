# Store Serialization Analysis and Plan

## Problem Statement

The current store implementation does not serialize writes from the point when a scru128 ID is generated to when the frame is written to storage and broadcast to subscribers. This creates a race condition where readers may observe frames with IDs that are not strictly increasing, violating the append-only store guarantee.

## Current Architecture

### Store Cloning and Concurrency

```rust
// src/store/mod.rs:177
#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    keyspace: Keyspace,
    frame_partition: PartitionHandle,
    idx_topic: PartitionHandle,
    idx_context: PartitionHandle,
    contexts: Arc<RwLock<HashSet<Scru128Id>>>,      // Shared via Arc
    broadcast_tx: broadcast::Sender<Frame>,          // Shared via Arc
    gc_tx: UnboundedSender<GCTask>,                  // Shared via Arc
}
```

- Store is `Clone` with Arc-wrapped shared state
- Each HTTP request gets a `store.clone()` (api.rs:451)
- Multiple concurrent requests access the same underlying storage

### Append Flow

```rust
// src/store/mod.rs:519-560
pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    frame.id = scru128::new();                    // [1] Generate ID

    // Context validation with RwLock
    if frame.topic == "xs.context" {              // [2] Validate
        self.contexts.write().unwrap().insert(frame.id);
    } else {
        let contexts = self.contexts.read().unwrap();
        if !contexts.contains(&frame.context_id) {
            return Err(...);
        }
    }

    if frame.ttl != Some(TTL::Ephemeral) {
        self.insert_frame(&frame)?;               // [3] Write to storage
    }

    let _ = self.broadcast_tx.send(frame.clone()); // [4] Broadcast
    Ok(frame)
}
```

### The Race Condition

**Scenario:** Two concurrent requests A and B

```
Time    Thread A                    Thread B
----    --------                    --------
T0      append() called
T1      ID=100 generated
T2                                  append() called
T3                                  ID=101 generated
T4                                  context validated
T5                                  insert_frame(101) ✓
T6                                  broadcast(101) ✓
T7      context validated
T8      insert_frame(100) ✓
T9      broadcast(100) ✓

Result: Subscribers receive [101, 100] - IDs out of order!
```

### Why This Matters

1. **Monotonicity Guarantee**: Append-only stores should guarantee strictly increasing IDs for readers
2. **Reader Confusion**: Clients using `last_id` for filtering (store/mod.rs:347) may miss frames
3. **Ordering Semantics**: Violates the fundamental contract of an ordered event log

## Proposed Solutions

### Approach 1: Mutex Around Entire Append

**Implementation:**
```rust
pub struct Store {
    // ... existing fields ...
    append_mutex: Arc<Mutex<()>>,
}

pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    let _guard = self.append_mutex.lock().unwrap();
    frame.id = scru128::new();
    // ... rest of append logic ...
    Ok(frame)
}
```

**Pros:**
- Simplest possible solution (3-4 lines of code)
- Guarantees strict ordering
- No algorithm changes needed
- Clear semantics: one append at a time

**Cons:**
- Serializes all appends completely
- Blocks concurrent appends during entire operation
- CAS writes (content-addressable storage) would block other appends
- Performance bottleneck for high-throughput scenarios

**Complexity:** ⭐ (Very Simple)

---

### Approach 2: Two-Phase with Separate ID Generation Lock

**Implementation:**
```rust
pub struct Store {
    // ... existing fields ...
    id_gen_mutex: Arc<Mutex<IdGenState>>,
}

struct IdGenState {
    last_id: Scru128Id,
    pending: VecDeque<Scru128Id>,
}

pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    // Phase 1: Reserve ID (fast, serialized)
    let (id, position) = {
        let mut state = self.id_gen_mutex.lock().unwrap();
        let id = scru128::new();
        state.pending.push_back(id);
        let position = state.pending.len() - 1;
        (id, position)
    };
    frame.id = id;

    // Phase 2: Write and broadcast (parallel, slow)
    let contexts = self.contexts.read().unwrap();
    // ... validation ...
    if frame.ttl != Some(TTL::Ephemeral) {
        self.insert_frame(&frame)?;
    }

    // Phase 3: Mark complete and broadcast in order
    {
        let mut state = self.id_gen_mutex.lock().unwrap();
        state.pending.remove(position);
        if position == 0 {
            // We're first, broadcast
            self.broadcast_tx.send(frame.clone());
            state.last_id = id;
        } else {
            // Wait for earlier frames
            // ... queue for later broadcast ...
        }
    }

    Ok(frame)
}
```

**Pros:**
- ID generation is fast and serialized
- Storage writes can happen in parallel
- Better throughput than Approach 1

**Cons:**
- Complex: need to track pending IDs and ordering
- Broadcasts must still be ordered, adding complexity
- Potentially unbounded queue of pending frames
- Deadlock risk if frame fails after ID generation

**Complexity:** ⭐⭐⭐⭐ (Complex)

---

### Approach 3: Async Channel with Single Writer Task

**Implementation:**
```rust
pub struct Store {
    // ... existing fields ...
    append_tx: UnboundedSender<AppendRequest>,
}

struct AppendRequest {
    frame: Frame,
    response_tx: oneshot::Sender<Result<Frame, Error>>,
}

pub fn append(&self, frame: Frame) -> Result<Frame, Error> {
    let (tx, rx) = oneshot::channel();
    self.append_tx.send(AppendRequest {
        frame,
        response_tx: tx
    })?;
    rx.blocking_recv()?
}

// Single writer task
fn spawn_append_worker(
    mut append_rx: UnboundedReceiver<AppendRequest>,
    store: Store,
) {
    tokio::spawn(async move {
        while let Some(req) = append_rx.recv().await {
            let mut frame = req.frame;
            frame.id = scru128::new();

            // All the append logic...
            let result = store.append_internal(frame);
            let _ = req.response_tx.send(result);
        }
    });
}
```

**Pros:**
- Clean separation: single point of serialization
- Similar pattern to existing GC worker (store/mod.rs:643)
- Natural backpressure via channel
- Easy to reason about

**Cons:**
- Requires splitting `append()` into public/internal
- Adds async/await complexity
- Need to handle channel errors
- Response channel adds overhead

**Complexity:** ⭐⭐⭐ (Moderate)

---

### Approach 4: RwLock with Write Guard for Append

**Implementation:**
```rust
pub struct Store {
    // ... existing fields ...
    append_lock: Arc<RwLock<()>>,
}

pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    let _guard = self.append_lock.write().unwrap();
    frame.id = scru128::new();
    // ... rest of append logic ...
    Ok(frame)
}
```

**Pros:**
- Slightly cleaner semantics than Mutex
- Same simplicity as Approach 1
- Could theoretically add parallel reads later (though not applicable here)

**Cons:**
- Same performance characteristics as Approach 1
- RwLock is overkill for this use case (no read path needed)
- Slightly more overhead than Mutex

**Complexity:** ⭐ (Very Simple)

---

### Approach 5: Atomic Counter + Post-Generation Ordering

**Implementation:**
```rust
pub struct Store {
    // ... existing fields ...
    append_counter: Arc<AtomicU64>,
    broadcast_ordering: Arc<Mutex<BroadcastQueue>>,
}

struct BroadcastQueue {
    next_expected: u64,
    pending: HashMap<u64, Frame>,
}

pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    // Get sequence number
    let seq = self.append_counter.fetch_add(1, Ordering::SeqCst);

    // Generate ID (guaranteed increasing by scru128)
    frame.id = scru128::new();

    // Write to storage (parallel)
    // ... validation and insert_frame ...

    // Ordered broadcast
    {
        let mut queue = self.broadcast_ordering.lock().unwrap();
        if seq == queue.next_expected {
            // We're next, broadcast immediately
            self.broadcast_tx.send(frame.clone());
            queue.next_expected += 1;

            // Drain any queued frames
            while let Some(f) = queue.pending.remove(&queue.next_expected) {
                self.broadcast_tx.send(f);
                queue.next_expected += 1;
            }
        } else {
            // Queue for later
            queue.pending.insert(seq, frame.clone());
        }
    }

    Ok(frame)
}
```

**Pros:**
- Storage writes can be parallel
- Only broadcast needs ordering
- Relatively simple logic

**Cons:**
- Still needs mutex for broadcast ordering
- Pending queue can grow if frames complete out of order
- Memory overhead for pending HashMap
- Doesn't fully solve the problem if storage writes themselves need ordering

**Complexity:** ⭐⭐⭐ (Moderate)

---

## Recommendation

### Recommended: Approach 1 (Mutex Around Entire Append)

**Reasoning:**

1. **Simplicity First**: The simplest solution that solves the problem completely
2. **Current Usage Patterns**:
   - Most appends come from HTTP API (relatively infrequent)
   - Handler appends (handler.rs) are not typically high-frequency
   - No evidence of high-concurrency append workloads
3. **Correctness Over Performance**:
   - Guarantees strict ordering with no edge cases
   - Easy to verify correctness
   - No complex state to debug
4. **Implementation Ease**:
   - ~5 lines of code change
   - No refactoring needed
   - Easy to test

### When to Reconsider

Move to **Approach 3** (Async Channel) or **Approach 5** (Atomic Counter) if:

- Profiling shows append contention is a bottleneck
- Append rate exceeds ~1000/sec consistently
- CAS writes become slow and block critical operations
- Need to support high-throughput event ingestion

### Implementation Steps for Approach 1

1. Add `append_mutex: Arc<Mutex<()>>` to Store struct
2. Initialize in `Store::new()`: `append_mutex: Arc::new(Mutex::new(()))`
3. Add lock guard at start of `append()`: `let _guard = self.append_mutex.lock().unwrap();`
4. Add concurrent append tests to verify ordering
5. Benchmark to establish baseline performance

### Testing Strategy

```rust
#[test]
fn test_concurrent_appends_are_ordered() {
    let store = Store::new(temp_dir());
    let handles: Vec<_> = (0..100)
        .map(|i| {
            let store = store.clone();
            std::thread::spawn(move || {
                store.append(Frame::builder("test", ZERO_CONTEXT).build())
            })
        })
        .collect();

    let frames: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().unwrap().unwrap())
        .collect();

    // Verify IDs are ordered when read
    let mut rx = store.read(ReadOptions::default()).await;
    let mut last_id = Scru128Id::default();
    while let Some(frame) = rx.recv().await {
        assert!(frame.id > last_id);
        last_id = frame.id;
    }
}
```

## Alternative: Do Nothing?

**Could we accept the current behavior?**

Arguments for status quo:
- SCRU128 IDs are globally sortable by timestamp
- Frames are stored atomically via fjall batches
- Out-of-order broadcasts are "rare" in practice

Arguments against:
- Violates append-only semantics
- Readers using `last_id` filtering can miss frames
- Debugging distributed systems becomes harder
- Future optimizations might make races more frequent

**Verdict**: The race condition is a real correctness issue that should be fixed. Even if rare, violating ordering guarantees in an append-only log is a fundamental problem.

## Failing Test

A test has been added to `src/store/tests.rs` that demonstrates the race condition:

```rust
#[tokio::test]
async fn test_many_concurrent_appends_maintain_ordering() {
    // Spawns 50 concurrent threads calling append()
    // Verifies that broadcast frames are in strictly increasing ID order
}
```

**Test Output:**
```
Frame 3 has ID 03ez5q26huint1lc9cxudzcmz <= previous ID 03ez5q26i5gdykepf79nfa4ty
```

The test reliably fails, proving that concurrent appends result in out-of-order broadcasts. This validates the problem analysis and provides a clear success criterion for any fix.

## Conclusion

Start with **Approach 1** (Mutex) for its simplicity and correctness guarantees. Monitor performance in production. If append contention becomes a bottleneck (unlikely given current usage), refactor to **Approach 3** (Async Channel) for better throughput while maintaining ordered semantics.

The failing test in `src/store/tests.rs:1312` should pass once serialization is implemented.
