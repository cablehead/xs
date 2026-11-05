# 0009: Supervisors catch up after falling behind

Supersedes one row of [ADR 0008](0008-follow-scan-first.md): supervisors no longer stop when they fall behind the live stream.

## Before and after

| a supervisor falls behind live (more than 4096 frames arrive faster than it reads) | before | after |
|---|---|---|
| what it does | stops with an error; no actor, service or action starts or stops until xs restarts | reads the frames it missed from the store, then carries on live |
| frames it misses | all of them, for good | none |
| frames it handles twice | none | none |

```
live read  ...  frame 900  |  fell behind: Err(Lagged)
reopen     read after frame 900 from the store  ->  901, 902, ...  ->  xs.threshold (dropped: already past it)  ->  live
```

## Why reopening is right here, and was not for actors

ADR 0008 rejected reopening a read for actors: an actor that writes a frame for every frame it reads creates its own backlog, so a reopened read can overrun again and again.

| reader | appends, per frame it reads | after reopening |
|---|---|---|
| actor writing per frame | about one | can fall behind again at once: keeps `fin.error` |
| supervisor | almost none (a lifecycle frame now and then) | the store's frames are a fixed backlog: it catches up |

Supervisors follow every topic and fall behind only in a burst of other writes. Once the burst ends, the frames to read are a fixed set in the store, and the supervisor reads them like any stored frames.

## How

`LifecycleReader` (`src/processor/mod.rs`), which all supervisors read through, opens its own read and remembers the last stored frame it handed out. On `Err(ReadError::Lagged)` it prints one line to stderr and opens the read again after that frame. The reopened read sends `xs.threshold` again; the reader drops it, since the supervisor is already past it. Heartbeats (`xs.pulse`) are not stored, so they are never a resume point.

Actors, services and the readers of ADR 0008's table keep ending with their error.
