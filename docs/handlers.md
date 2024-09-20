# Handlers Documentation

## Overview

Handlers allow you to register **Nushell closures** that respond to events.
Handlers can be **stateless** or **stateful**:

- **Stateless**: Executes the closure for each event but doesn't maintain state
  between events.
- **Stateful**: Maintains and updates state across events.

Closures and metadata are passed directly as Nushell objects — no need to quote
them.

## How to Register a Handler

To register a handler, pipe a **Nushell closure** into `.append`. Metadata is a
**Nushell record** provided with the `--meta` flag when customization is needed.

### Command Syntax:

```nushell
{|| <closure>} | .append <topic>.register [--meta <metadata-record>]
```

- `<topic>`: The event topic the handler listens to.
- `[--meta <metadata-record>]`: Optional Nushell record to define behavior (see
  below).
- `{|| <closure>}`: Nushell closure that processes the event.

### Example:

```nushell
{|| if $in.topic == 'foo' { 'handled event' }} | .append foo.register
```

This example registers a stateless handler that listens for events on the `foo`
topic.

## Metadata Options

| Option          | Type                | Description                                                                                                               |
| --------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `stateful`      | `bool` (optional)   | `true` to maintain state across events. Default is `false`.                                                               |
| `initial_state` | `record` (optional) | The initial state for stateful handlers.                                                                                  |
| `pulse`         | `number` (optional) | Time in milliseconds for heartbeats.                                                                                      |
| `start`         | `record` (optional) | Specifies where to start reading (e.g., `{ head: "id" }`). If not provided, the handler starts at the tail of the stream. |

### Default Start Behavior

If `start` is not provided, the handler begins processing from the **tail** of
the stream (i.e., only new events that occur after the handler is registered
will be processed).

## Stateless vs. Stateful Handlers

- **Stateless**:
  - Does not store state between events.
  - **Return values**: If the return value is `null`, nothing happens.
    Otherwise, the return value is **stored** for the current event but is not
    used in future executions.

- **Stateful**:
  - Maintains and updates state across events.
  - **Return values**: The return value must contain updated state. This state
    is stored and passed into future invocations.

## Examples

### Stateless Handler Example

```nushell
{|| if $in.topic != 'topic2' { return } 'ran action' } | .append action.register
```

- Listens for the topic `"topic2"`.
- **Return value**: Stored for the event but not retained for future events.

### Stateful Handler Example

```nushell
{|state| $state.count += 1; { state: $state } } | .append counter.register --meta {stateful: true, initial_state: {count: 0}}
```

- Tracks a count of how many times `"count.me"` is triggered.
- **Return value**: Updates the `count`, and the new state is used in the next
  event.
