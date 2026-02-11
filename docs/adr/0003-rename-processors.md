# ADR 0003: Rename Processors to Service, Actor, Action

## Context

The three processing types were called "generator", "handler", and "command". All three names are generic, overloaded in programming, and don't describe what they actually do in cross.stream. "Handler" and "command" especially clash -- "handler" is vague, and "command" collides with the built-in store commands (`.append`, `.cat`, etc.).

Nushell's `generate` command is a stateful closure over a stream -- exactly what our "handler" does. Our "generator" is really a managed external process. The names were effectively swapped relative to Nushell's vocabulary.

## Decision

Rename the three types:

| Old | New | Why |
| --- | --- | --- |
| generator | **service** | Long-running, auto-restarts, managed lifecycle -- like systemd services |
| handler | **actor** | Receives frames, maintains state, emits output -- the actor model |
| command | **action** | Stateless, on-demand, request-response -- something you trigger |

The umbrella term for all three is **processor**. A processor is anything that runs closures against the store with a managed lifecycle. The store is passive; processors are active.

Topic suffixes stay the same (`.spawn`, `.register`, `.define`, etc.).

## Consequences

- Naming aligns with Nushell: users familiar with `generate` will recognize the actor pattern
- "Service" communicates the operational character (long-running, restarts) not just the data direction
- "Action" stops colliding with built-in store commands
- "Processor" replaces the informal use of "component" in docs and code
- Code modules rename: `generators/` -> `service/`, `actor/` -> `actor/` (stays), `commands/` -> `action/`
