# XS Naming Schema - Quick Reference Guide

*For the full detailed rationale and migration plan, see [NAMING_SCHEMA.md](./NAMING_SCHEMA.md)*

---

## At-a-Glance Summary

### The Big Changes

| What | Old | New | Reason |
|------|-----|-----|--------|
| Skip to latest | `--tail` | `--from-latest` | Clearer semantics |
| Resume from point | `--last-id` | `--from-id` | More explicit |
| Start from oldest | (missing) | `--from-beginning` | Consistency |

### Why?

The old terms were **confusing**:
- `--tail`: Sounds like "the end" but actually means "skip to latest" (Git vs Unix semantics)
- `--last-id`: Unclear if it's "resume from this" or "exclude up to this"

The new terms are **explicit**:
- `--from-latest`: Starts reading from the latest frame
- `--from-id`: Resumes from a specific frame
- `--from-beginning`: Includes all frames from the oldest

---

## Core Concepts

### Frame
```
A single immutable event in the stream.
Example: A login attempt, a payment, a system alert

Frame {
  id: 0c8z7k...,              # Unique identifier
  topic: "accounts:auth:login", # Subject/category
  context_id: user-123,         # Isolation scope
  hash: sha256:abc...,          # Content hash
  meta: { user: "bob" },        # Optional metadata
  ttl: 5000ms,                  # Expiration
}
```

### Topic
```
Subject line for organizing frames.
Hierarchical naming: domain:entity:event-type

Good examples:
- accounts:user-auth:login-success
- payments:transaction:completed
- systems:health:cpu-alert
```

### Context
```
Isolation boundary / namespace.
Usually: per-user, per-job, or system-wide

Examples:
- System context: ZERO_CONTEXT (for system operations)
- User context: "user-123"
- Job context: "job-456"
```

### Stream
```
Ordered append-only log of frames.
One stream per (topic, context) pair.

You read/write frames, not streams.
Streams are a logical organization.
```

---

## CLI Cheat Sheet

### Reading Frames

```bash
# Latest frame in a topic
xs head addr topic

# All frames from oldest to newest
xs cat addr --from-beginning

# All frames from latest onward (new ones only)
xs cat addr --from-latest

# Resume from specific frame
xs cat addr --from-id 0c8z7k...

# Keep watching for new frames
xs cat addr --follow

# Limit results
xs cat addr --limit 10

# Filter by topic
xs cat addr --topic "accounts:*"

# Specific context
xs cat addr --context user-123

# All contexts at once
xs cat addr --all
```

### Writing Frames

```bash
# Append data to topic
xs append addr my-topic < data.json

# With metadata
xs append addr my-topic --meta '{"user":"bob"}' < data.json

# With TTL
xs append addr my-topic --ttl forever < data.json
xs append addr my-topic --ttl ephemeral < data.json
xs append addr my-topic --ttl time:5000 < data.json
xs append addr my-topic --ttl head:10 < data.json

# To specific context
xs append addr my-topic --context user-123 < data.json
```

### Other Operations

```bash
# Get frame by ID
xs get addr 0c8z7k...

# Remove frame
xs remove addr 0c8z7k...

# Get content by hash
xs cas addr sha256:abc...

# Store content
xs cas-post addr < data.bin
```

---

## API Parameter Mapping

### Query Parameters

```
GET /?from-id=ID              # Resume from frame
GET /?from-latest=true        # Skip existing
GET /?follow=true             # Subscribe
GET /?limit=10                # Max results
GET /?topic=pattern           # Topic filter
GET /?context-id=ID           # Context filter
GET /?all=true                # All contexts

POST /append/{topic}?context=ID&ttl=SPEC
POST /append/{topic}?context=ID&ttl=forever
```

### Headers

```
xs-meta: base64(json)         # Frame metadata on POST
Accept: text/event-stream     # For SSE format
```

---

## Naming Rules

### Character Set
```
Allowed: a-z, 0-9, hyphen (-), underscore (_)
Not allowed: uppercase, space, period, slash, special chars
Max length: 253 characters
Must start and end with alphanumeric
```

### Topic Naming Examples

Good:
```
domain:entity:event-type
accounts:user-auth:login-success
payments:transaction:completed
systems:health:cpu-spike
user:profile:updated
```

Bad:
```
User.Auth.Login     # Periods, wrong case
login_success       # Missing domain
xs_internal         # Mixed separators
```

---

## Common Patterns

### Listening for new frames only
```bash
# Skip everything in buffer, show only new
xs cat addr --from-latest --follow
```

### Resuming a subscription
```bash
# Where you left off
xs cat addr --from-id $LAST_FRAME_ID --follow
```

### Getting the latest state
```bash
# Most recent frame in a topic
xs head addr my-topic --follow  # With updates
xs head addr my-topic            # Just latest
```

### Replaying history
```bash
# All frames from the beginning
xs cat addr --from-beginning --topic my-topic
```

### Limiting results
```bash
# Get exactly 10 frames
xs cat addr --limit 10 --from-beginning
```

### Cross-context queries
```bash
# All frames across all contexts
xs cat addr --all

# Specific context only
xs cat addr --context user-123
```

---

## Deprecation Status

### Currently Supported (Old Names)
- `--tail` → Use `--from-latest` instead
- `--last-id` → Use `--from-id` instead

These will be removed in a future version. Use new names for new code.

---

## Implementation Status

- [x] Naming schema designed
- [ ] Code refactoring
- [ ] Documentation updates
- [ ] Release notes
- [ ] Migration guides

See [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) for full migration plan.

---

## Questions?

- Discord: [xs Discord](https://discord.com/invite/YNbScHBHrh)
- Issues: GitHub Issues
- Docs: Full reference in NAMING_SCHEMA.md
