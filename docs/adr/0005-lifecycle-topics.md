# ADR 0005: Lifecycle Topics

Unified topic vocabulary and namespace for actor, service, and action
lifecycles, and the compaction algorithm that consumes them.

Supersedes the per-processor ad-hoc lifecycle vocabulary established
implicitly by [ADR 0003](./0003-rename-processors.md).

## Context

Today actor, service, and action each carry a different ad-hoc lifecycle
vocabulary, and all three sit in the app's topic namespace:

- Actor: `<name>.register` / `<name>.unregister` (in), `<name>.active` /
  `<name>.unregistered` (out). One `.unregistered` covers every stop
  reason, distinguished only by meta.
- Service: `<name>.spawn` / `<name>.terminate` (in), `<name>.running` /
  `<name>.stopped` / `<name>.parse.error` / `<name>.shutdown` (out).
  `.stopped` carries `meta.reason in {finished, error, terminate, update}`.
- Action: `<name>.define` / `<name>.call` (in), `<name>.ready` /
  `<name>.error` (out). `.error` overloads parse failure and runtime
  failure. No undefine path.

Each dispatcher's startup compaction logic differs in shape, not just
in topic strings, and the runtime's own lifecycle frames sit in the same
namespace as user data.

### Deficiencies

Eight concrete bugs surface in the current implementation. Each is
referenced by the invariant set below.

1. Service `.stopped {reason=finished}` and `.stopped {reason=error}`
   restart on boot. Contrary to "if it chose to stop, it stays stopped."
2. Service hot-replace where the new `.spawn` has a parse error: old
   keeps running live (correct), but compaction is latest-wins so the
   broken `.spawn` survives and the service vanishes on next boot.
3. Action hot-replace + parse error: same shape as #2. Broken `.define`
   overwrites; previously-good define is lost on restart.
4. Action with a broken `.define` retries the broken version on every
   boot. Dispatcher doesn't scan `.error` historically; no "skip broken"
   path.
5. Actor hot-replace + parse error kills *both*: the old instance
   self-terminates on any duplicate `.register` before the new is
   validated; the new then parse-fails. Net: nothing running.
6. Action has no undefine. Once defined, the only way to remove is to
   edit history.
7. Action `.error` overloads parse failure (`register_action` Err) and
   runtime failure (per-call `execute_action` Err); the two cannot be
   told apart at the topic level.
8. Actor `.unregistered` overloads parse failure with graceful
   teardown; only `meta.error`'s presence distinguishes them.

### Performance pressure

To find "all actor lifecycle events," the runtime has to scan every
frame in the store and filter by suffix, an O(stream) cost that no
index helps. On a 110k-frame store this drives `~17us/frame *
110k = ~1.9s` per dispatcher start, plus `~1.5s` per actor inside
`build_engine` (each actor independently re-walks the store via
`nu_modules_at`).

## Decision

### Namespace

Lifecycle frames live under `xs.`, a namespace owned by the runtime.
User-chosen data topics stay where they are.

```
xs.actor.<name>.<event>         actor lifecycle
xs.service.<name>.<event>       service lifecycle
xs.action.<name>.<event>        action lifecycle
xs.module.<name>                module registration (replaces <name>.nu)

<name>.recv / .send / .out      app-level data, runtime injects nothing
<anything-not-xs.*>              app-owned, runtime ignores
```

Glance test: a topic starting with `xs.` is runtime-managed; everything
else is app data.

Runtime queries become pure prefix scans on the existing hierarchical
`idx_topic` index. No new index keyspace, no suffix matcher, no
schema-version bump for indexing:

| Query | Prefix |
|---|---|
| all system events | `xs.` |
| all actor lifecycle, every actor | `xs.actor.` |
| one actor's lifecycle | `xs.actor.snapshot-actor.` |
| all modules | `xs.module.` |
| one module's history | `xs.module.game.` |

At startup, each dispatcher reads only the frames in its own namespace,
a few hundred, not 110k.

### Lifecycle vocabulary

Apply uniformly across actor, service, and action (action uses a
subset). `in` = user-appended, `out` = runtime-emitted. The `<event>`
segments:

| Event | Dir | Meaning |
|---|---|---|
| `create` | in | user wants this thing running |
| `term` | in | user wants this thing stopped |
| `active` | out | runtime is up; `meta` points at the originating `create` |
| `invalid` | out | source failed to parse (or any other init-time validation); `meta` points at the originating `create` |
| `fin.error` | out | runtime crashed |
| `fin.ok` | out | task ran to natural completion |
| `fin.term` | out | exited because of `term` |
| `replaced` | out | exited because a newer `create` won (transient marker) |
| `stopped` | out | exited because of `xs.stopping` (server shutdown) |

The `fin.*` family means "terminal, will not restart." `replaced` and
`stopped` are outside the family because they describe stops that
should *not* affect restart: `replaced` because a successor is coming;
`stopped` because the server itself is coming back.

### Compaction algorithm

Track two slots per `<kind>.<name>`:

- `confirmed`: last `create` that emitted `active`.
- `pending`: latest `create` with no terminal ack yet.

State transitions:

| Frame | Effect |
|---|---|
| `create` | `pending = this` |
| `active(source=X)` | `confirmed = create-X`; clear `pending` if it points at X |
| `invalid(source=X)` | clear `pending` if it points at X |
| `term` | clear both |
| `fin.*` (error / ok / term) | clear both |
| `replaced` | no effect |
| `stopped` | no effect |

At threshold:

```
if pending:    try pending; on parse-fail, fall back to confirmed
elif confirmed: start confirmed
else:          nothing to start
```

### Cases the algorithm handles

- **Hot-replace race** (`create_1 -> active_1 -> create_2 -> ???` and xs
  dies): on restart, `confirmed=create_1`, `pending=create_2`. Try
  `create_2`; on fail, fall back to `create_1`.
- **Hot-replace, broken replacement** (`invalid_2` lands live):
  `pending` cleared, `confirmed=create_1` survives. Old version restarts
  on boot.
- **Hot-replace success during transition window**: `replaced` does not
  clear `confirmed`, so the fallback survives the brief gap between
  old's `replaced` and new's `active`. The replacement's `active`
  overwrites `confirmed` cleanly.
- **First create, never acked** (xs died before processing): `pending`
  set, `confirmed` empty. Try `pending`. If it succeeds, advance; if it
  fails, nothing to fall back to (correct).
- **Server crash mid-run**: `confirmed` set, `pending` empty. Start
  `confirmed`. Service was running fine, server crash should resume.
- **Server shutdown**: `stopped` doesn't affect compaction; `confirmed`
  persists; service resumes on next boot.
- **User `term` while xs offline is impossible** (fjall is
  single-writer), but `term` appended in a prior live session clears
  both slots, so the thing stays down on next boot.

### Property: ack-independence of `term` and `fin.*`

`term` and `fin.*` clear both compaction slots on observation. The
algorithm never waits for a paired ack. If `term` is in the log but xs
died before processing it (so no `fin.term` was emitted), the `term`
alone keeps the thing stopped on the next restart.

### Invariants

The compaction algorithm and topic vocabulary above exist to honor
these contracts. Each one is testable; together they cover every
deficiency in the previous section.

- **I1. Stop persistence.** Once `term` or any `fin.*` has been observed
  for a `<kind>.<name>`, no subsequent restart starts the prior
  `create`.
- **I2. Run persistence.** A `<kind>.<name>` with an `active` and no
  subsequent `fin.*`/`term` resumes on every restart until something
  terminal lands.
- **I3. Hot-replace fallback.** When a newer `create_2` follows a
  known-good `create_1`, and `create_2` is broken (`invalid`) or
  untested (no ack), restarts fall back to `create_1`. Live behaviour
  and post-restart behaviour agree.
- **I4. Bidirectional lifecycle.** Every kind supports a user-driven
  `term` that ends the thing and prevents restart.
- **I5. Distinct exit categories.** The topic alone (no meta needed)
  distinguishes: failed-to-init vs user-terminated vs runtime-crashed
  vs naturally-finished vs replaced vs server-shut-down.
- **I6. Ack traceability.** Every runtime-emitted ack (`active`,
  `invalid`, `fin.*`, `replaced`) carries a meta pointer to its
  originating `create` (or `term`).
- **I7. Server-shutdown invisibility.** A `stopped` event does not
  affect compaction; the thing resumes on next start.
- **I8. Single live instance.** At most one running instance per
  `<kind>.<name>` at any time.

### Coverage check

Each enumerated deficiency would be caught by a test of the named
invariant:

| # | Deficiency | Caught by |
|---|---|---|
| 1 | Service `.stopped {finished/error}` restarts on boot | I1 |
| 2 | Service hot-replace + parse error: old version lost on restart | I3 |
| 3 | Action hot-replace + parse error: same | I3 |
| 4 | Action broken `.define` retries every boot | I3 |
| 5 | Actor hot-replace + parse error kills both | I3 |
| 6 | Action has no undefine | I4 |
| 7 | Action `.error` overloads parse and runtime failure | I5 |
| 8 | Actor `.unregistered` overloads parse-failure with graceful teardown | I5 |

I6 and I8 don't catch one of the enumerated deficiencies directly, but
the invariants that do depend on them: without ack traceability (I6)
you can't pair `invalid` to its `create`, so I3 is unenforceable;
without single-instance (I8) you can't unambiguously define "the
thing" that I1/I2 track. They stay in the set as supporting
invariants.

I7 is implied by I2 + I1 (`stopped` isn't `fin.*` so it doesn't
satisfy I1's "stop observed"), but stating it explicitly closes a
likely misreading of `stopped` as a terminal event.

### Action subset

Actions don't run long-lived tasks. The events they use:

- `create` (was `.define`), `term` (new, adds the missing undefine),
  `active` (was `.ready`), `invalid`, `fin.term` (on user undefine),
  `fin.replaced` (on re-define), `replaced` (transient).
- No `fin.ok` (actions don't naturally finish), no `fin.error` at the
  *lifecycle* level (per-invocation runtime errors stay on the app's
  per-call response topic, not in the action's lifecycle stream), no
  `stopped` (actions don't run during `xs.stopping`).

## Breaking change

This is a breaking change. xs is pre-1.0; there is no migration path
and the runtime does not attempt to read pre-rename frames. Stores
written by pre-rename xs binaries are not supported. Users with
existing data should either:

- Start fresh.
- Stay on a pre-rename release of xs.
- Roll their own conversion (the topic-name mapping is documented
  below for that purpose).

The mapping between old and new topic shapes, for anyone writing a
custom conversion:

```
snapshot-actor.register      -> xs.actor.snapshot-actor.create
snapshot-actor.active        -> xs.actor.snapshot-actor.active
snapshot-actor.unregistered  -> xs.actor.snapshot-actor.fin.{term|error|ok}
                                (split by meta.reason / meta.error presence)
api.spawn                    -> xs.service.api.create
api.terminate                -> xs.service.api.term
api.stopped (reason=...)     -> xs.service.api.fin.{ok|error|term} or .replaced
api.shutdown                 -> xs.service.api.stopped
api.parse.error              -> xs.service.api.invalid
greet.define                 -> xs.action.greet.create
greet.ready                  -> xs.action.greet.active
greet.error                  -> xs.action.greet.invalid  (parse cases only)
game.nu                      -> xs.module.game
```

## Consequences

- Three dispatchers share one compaction algorithm template; only the
  per-kind prefix differs.
- No new index keyspace, no suffix matcher: the existing
  `idx_topic_prefix_keys` hierarchical index serves every runtime
  query in O(matches).
- Dispatcher cold-start drops from "scan whole stream for lifecycle
  topics" to "prefix-scan `xs.<kind>.`". On the 110k-frame measurement
  the historical phase drops from ~1.9s to milliseconds.
- Action gains an undefine (`term`) and a real lifecycle vocabulary,
  closing the gaps from the audit (broken `.define` retried every
  boot; `.error` overloading parse + runtime failure).
- Pre-rename data is not readable. Users on existing stores must start
  fresh or stay on an older xs release.
