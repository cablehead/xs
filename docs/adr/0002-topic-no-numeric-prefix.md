# ADR 0002: Disallow Numeric Prefix in Topic Names

Supersedes the "Must start with" rule in [ADR 0001](./0001-hierarchical-topic-index.md).

## Context

`.last` takes a `-n` flag for count: `.last topic -n 5`. Positional args (`.last [topic] [number]`) are simpler, but `.last 42` is ambiguous if topics can start with digits.

## Decision

Topics must start with `a-z A-Z _`. A leading digit means "count", a letter or underscore means "topic".

## Consequences

- `.last [topic] [number]` works without flags
- `123`, `42foo` become invalid topics (no known usage)
- Numeric segments after the first dot still fine (`orders.2024.pending`)
