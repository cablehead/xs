# 0008: Replica stores

A remote xs store can be mirrored locally as a read-only replica, addressed
as another log alongside the default store, so every read-side primitive
(`.cat`, `--follow`, `.last`, topic filters, TTL semantics) works on it
unchanged.

## Model

Hypercore-style: one writer per stream. A replica is not a synced copy of a
log file, it is **a store with a live broadcast**. The replicator is itself
a follower of the remote (`xs cat <remote> --follow`), and re-broadcasts
what it reads locally without persisting ephemeral frames. A live follow on
the replica therefore sees ephemeral frames the same as a local follow does
-- they never touch either side's disk, only the two broadcast channels
back to back.

## Storage: cores, not stores

Today's layout is `<store>/fjall/` (keyspace `stream` + `idx_topic`) plus
`<store>/cacache/`. A replica is **another keyspace pair in the same fjall
database**: `stream.<name>` / `idx_topic.<name>`, opened lazily via
`Store::core(name)`. It shares the parent store's directory, so it shares
its CAS -- hash is identity, so a blob referenced by both the default store
and a replica is stored once.

`Store::core` caches the opened keyspace pair, broadcast channel, GC worker
and append lock in a registry (`Store.cores`) shared by every clone of the
opening store. This is why the registry has to exist at all, rather than
just reopening the fjall keyspace handles per call: a replica follow started
before the replicator connects must still observe frames the replicator
later writes, which means it has to be subscribed to the *same*
`broadcast::Sender` the replicator eventually sends on.

A `Store` value returned by `core()` is structurally the same type as the
default store and shares the same read path (`read`, `read_sync`, `get`,
...) -- "every piece of machinery that works on the local stream works on
the replica" falls out of that for free, rather than needing a parallel
implementation.

## Addressing: `<addr>/<name>`

`xs cat <addr>/vm --follow` and every other CLI verb select a core by a
trailing path segment on the address, resolved once in
`client::RequestParts::parse` and forwarded as a `xs-core` request header
(`src/api.rs::CORE_HEADER`). Every existing flag (`--follow`, `--after`,
`-T`, `--last`, `--sse`, ...) is unaffected: it's the same route dispatch
against a different `Store::core(name)` handle in the HTTP handler.

Splitting the trailing segment differs per transport:

- **TCP/TLS**: `host:port/vm` -- the URL path (previously parsed and
  discarded) is the core name.
- **iroh**: `iroh://<ticket>/vm` -- split on the first `/` after the scheme.
- **Unix socket**: ambiguous, because the address is itself a filesystem
  path made of `/`-separated segments, and a not-yet-started store's
  directory doesn't exist yet to check against. Resolved by sniffing for
  `<dir>/fjall`, the one thing that's true of every xs store directory and
  false of an arbitrary path segment: if `addr` itself looks like a store
  (or is an explicit `sock` file), there's no core; otherwise, if `addr`'s
  *parent* looks like a store, the last segment is the core name; otherwise
  there's no core and `addr` is passed through unchanged (so `xs cat
  ./not-started-yet` still reports "no store at", instead of misreading
  `not-started-yet` as a core name of `.`).

`xs.nu`'s `.cat` gained an optional positional `core` argument that appends
`/<core>` onto `(xs-addr)` before shelling out, so `.cat vm --follow` reads
the replica from a nushell session the same way `xs cat <addr>/vm --follow`
does from the shell.

`/eval` is rejected (405) for an addressed core, same as the other mutating
routes, even though a read-only script would be harmless: `eval_engine`
builds its nu engine's `.append`/`.import`/`.remove` builtins bound to
whatever `Store` it's given, so scoping `/eval` to a core would hand a
script exactly the write access the rest of this design goes to lengths to
deny. The in-process `.cat` builtin (`CatCommand`, used inside `xs
eval`/service/actor engines) is unaffected by any of this either way -- it
reads whichever `Store` its engine was built with, and this task doesn't add
a way to hand it a different core. Scripts that need a replica's frames from
inside an engine can still shell out via the `xs` binary; wiring
core-selection into the in-process builtin (with the same read-only
enforcement `/eval` needs) is future work if that turns out to matter.

## Write paths

|              | mints id | stores           | broadcasts |
|--------------|----------|------------------|------------|
| `append`     | yes      | unless ephemeral | yes        |
| `import`     | no       | always           | no         |
| `replicate`  | no       | unless ephemeral | yes        |

`replicate_frame` is a third path (`Store::replicate_frame`), sharing its
body with `append` (`Store::write_frame`) minus the id-minting line:
replication has to preserve the origin's id and hash -- that's what makes
CAS dedup by hash meaningful across cores, and what makes "the replica's own
last frame id" a valid resume cursor (below) -- while still observing TTL
storage rules and broadcasting so a replica follow sees ephemerals.

`replicate_frame` is not reachable over HTTP; only the in-process replicator
task (below) calls it. Read-only enforcement is mechanical but lives at the
HTTP boundary, not inside `Store`: `api::handle` checks for the `xs-core`
header and returns 405 on `/append/*`, `/import`, `DELETE /<id>`, and
`/eval` for *any* addressed core, before the route even runs. `Store::append`,
`Store::insert_frame`, and `Store::remove` stay generic over which keyspace
they operate on -- deliberately, because GC (`Store::remove`, driven by TTL
expiry) has to work identically on a replica core to honor "ttl semantics
... differing only in being read-only", and gating removal by core would
have broken that. The single-writer guarantee is a property of what's
reachable over the wire, not of what the Rust API allows a trusted, in-
process caller to do.

## Supervised task: `xs.replica.<name>`

Declared in-stream, reusing the lifecycle vocabulary from ADR 0005 as-is:

    xs.replica.<name>.create {addr: "<remote>"}   ->  .active  ->  .fin.term | .stopped

`processor::replica::serve::run` is a dispatcher shaped exactly like
`processor::service::serve::run`: the same `Slots`/`LifecycleReader`
compaction machinery, restart-on-boot behavior, and one JoinHandle per name
in an `active` map. `processor::replica::replica::run` is the per-replica
task: it resolves `meta.addr` (a bad or missing address is `.invalid`, not
a panic), opens `store.core(name)`, sets that core's replication origin,
emits `.active`, and then loops: connect to the remote with
`client::cat_frames(addr, follow, after: <cursor>)`, forward every frame
to `core_store.replicate_frame`, reconnect with backoff on disconnect.
It watches its own `.term` topic and `xs.stopping` via a second, local
subscription (the same pattern `service`'s run loop uses for its control
channel) and exits on either, emitting `.fin.term` or `.stopped`
respectively.

**Cursor.** No separate durable-cursor keyspace: a replica core's own
`stream.<name>` keyspace already stores frames under their *origin* ids, in
order (`replicate_frame` doesn't remint them), so "the last frame this core
has" and "the replication cursor" are the same value. Resume-after-restart
is `core_store.read_sync(last: 1)`. This is the same trick a plain follower
uses to resume its own position -- no new mechanism, just recognizing the
replica's own log already encodes it.

**CAS.** Never pulled eagerly. `Store::cas_reader`/`cas_read` fall back to
fetching from `replica_origin()` on a local cache miss, writing the result
into the shared CAS before serving it. Safe to leave blobs pulled this way
in place forever: there is no CAS GC in this codebase (`GCTask` only ever
targets the frame index -- `Remove`/`CheckLastTTL`/`Drain` -- never
`cacache`), so nothing needs to track when a pulled blob is safe to evict.
The one gap: the CLI's direct-filesystem fast path for local Unix-socket
CAS reads (`client::cas_get`, added for same-machine performance before this
task) bypasses the HTTP layer entirely and so bypasses this fallback too:
it's disabled specifically when a core is addressed, falling through to the
normal HTTP path instead.

## Left open

- **No hot-replace for a replica's `addr`.** A second `.create` for a name
  that's already running is ignored by the dispatcher (same as `service`
  today). Changing which remote a running replica points at requires
  `.term` then a fresh `.create`. Fine for now; add hot-replace the same way
  `service` did if it's needed.
- **Frame identity across the boundary is unsigned.** Replication preserves
  ids, but nothing signs a core -- a replica trusts whatever the socket
  said. Fine while the transport itself is the trust boundary; revisit if a
  replica is ever relayed second-hand (noted in `xs-replica-panes.md`).
  Recommendation if/when that's needed: sign at the frame level (a
  per-writer key, checked on replicate) rather than only at the transport
  level, so a relayed replica can still be verified against its original
  writer. Not implemented here -- out of scope for a same-network replica.
- **In-process `.cat` builtin has no core selector.** See "Addressing"
  above. Recommendation if this becomes necessary: thread an optional
  `Store` override through the nu engine's command registration rather than
  a per-call argument, since which store a builtin reads from is normally
  fixed at engine-build time in this codebase (ADR 0006/0007), not a
  runtime parameter.
