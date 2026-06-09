# 0006: Store builtins on a prepared, cloneable engine

Each runner kind (eval, service, action, actor) builds one prepared base engine and clones it per spawn. No runner re-registers builtins after the base exists.

## Base

`nu::prepared_base(store, read, direct_write)` = nushell + stdlib + the core builtins + `.rm` alias + the read builtins, plus `.append` for direct writers. `build_engine` is gone. Per use, the caller clones the base, applies `load_modules(as_of)`, and sets per-instance state.

| runner  | read   | write                              |
|---------|--------|------------------------------------|
| eval    | Stream | Direct `.append` on the base       |
| service | Stream | Direct `.append` on the base       |
| action  | Stream | Direct `.append` on the base       |
| actor   | Plain  | Buffered `.append` added per clone |

## Append modes

`AppendMode` is the one builtin whose behaviour varies by runner:

- **Direct** -- each `.append` writes its frame straight to the store as the call runs. Used by eval, service, and action. The decl carries no per-instance state, so it lives on the shared base.
- **Buffered** -- `.append` collects frames into a per-instance buffer instead of writing; the actor flushes that buffer so an actor's appends land atomically alongside its output frame. The buffer is per-instance, so this `.append` is added to each clone, not the base.

Both modes share the same `.append` signature and the `XS_APPEND_META` base metadata below; they differ only in where the frame goes.

## .append metadata

`.append`'s base metadata is instance-independent: it comes from the `XS_APPEND_META` env var (a JSON object string), read at run time, not from the command constructor. The runner sets it per instance via `Engine::set_append_meta`:

- service: `{service_id}`
- action: `{action_id, frame_id}` (per triggering frame)
- eval: unset, so the base is empty

`.append --meta {..}` merges over the env base; an absent or malformed env resolves to an empty base.

## Why

Baking metadata into the decl forced every spawn to re-register `.append`, and the service hot-replace path forgot to, so a re-registered service hit `.append command not found`. With the builtins on a cloned base and metadata in `$env`, a hot-replace clones a pristine engine and cannot lose `.append`.
