# XS Project Naming Schema - Comprehensive Guide

*This document establishes consistent naming conventions for the xs project, aligned with industry best practices from Git, NATS, Kafka, Redis, and Kubernetes.*

## Executive Summary

The xs project has inconsistencies in naming conventions across several domains:
- **Temporal navigation**: `head` vs `last` vs `first` (unclear semantics)
- **Stream boundaries**: `tail` with confusing meaning ("begin long after the end")
- **Context/Index terminology**: `context_id` vs `context` (mixing levels of abstraction)
- **Command naming**: Inconsistent between CLI, API, and internal code

This guide establishes clear, consistent naming conventions that follow industry best practices and eliminate ambiguity.

---

## Part 1: Current State Analysis

### Enumerated Concepts in XS

#### Data Structures
- **Frame**: Individual event/record in the stream (analogous to Git commit, Kafka message)
- **Stream**: Append-only sequence of frames (analogous to Git DAG, Kafka topic)
- **Topic**: Subject/category for organizing streams (analogous to Git branch intent, Kafka topic)
- **Context**: Isolation boundary for a stream of frames (namespace/scope)
- **Index**: Lookup/access point (topic index, context index)
- **ID**: Unique identifier using SCRU128 format

#### Operations
- **Append**: Add a frame to the stream
- **Head**: Get the most recent frame (in Git: HEAD points to latest commit)
- **Tail**: Create subscription starting from now (semantic confusion: "begin long after the end")
- **Cat**: Read/concatenate all frames matching criteria
- **Get**: Retrieve a specific frame by ID
- **Remove**: Delete a frame from the stream
- **Follow**: Watch for new frames in real-time

#### Configuration/Parameters
- **follow**: Subscribe mode (On/Off/WithHeartbeat)
- **tail**: Current naming means "start from latest" but Git/Unix conventions conflict
- **last-id**: Resume from specific frame
- **limit**: How many frames to retrieve
- **context-id**: Which isolation boundary to operate within

### Current Inconsistencies Found

1. **Head/Tail Semantic Confusion**
   - `.head` = get most recent frame (matches Git: HEAD points to latest)
   - `.tail` = "begin long after the end of the stream" (confusing; matches Unix tail semantics)
   - Problem: Users confused because "tail" doesn't mean "the end of the stream"
   - Problem: Git and Unix have opposite conventions

2. **First/Last vs Head/Tail**
   - Suggested alternatives: `.first` (oldest), `.last` (newest)
   - Problem: Need to clarify inclusive vs exclusive boundaries

3. **Context/Index Terminology Overloading**
   - `context_id`: The specific context identifier
   - `context`: Sometimes means "current context", sometimes "context parameter"
   - `idx_context`: Internal index for contexts
   - `idx_topic`: Internal index for topics
   - Problem: "Context" and "Index" are mixed levels of abstraction

4. **Command Structure Inconsistency**
   - CLI: `.cat`, `.append`, `.head` (object.method style)
   - API Routes: `StreamCat`, `StreamAppend`, `HeadGet` (CamelCase compound)
   - Internal: `cat()`, `append()`, `head()` (simple function names)
   - Problem: Different naming conventions across layers

5. **Boolean Flag Naming**
   - `--tail` is actually "start from latest" (should be `--from-latest`?)
   - `--follow` is "watch for updates" (clear)
   - Problem: flag semantics not obvious from name

---

## Part 2: Industry Best Practices Research

### Git Naming Conventions
**Source**: [Git References Hierarchy - Baeldung](https://www.baeldung.com/ops/git-refs-branch-slash-name), [Atlassian Git Refs](https://www.atlassian.com/git/tutorials/refs-and-the-reflog)

Key principles:
- **HEAD**: Special ref pointing to current branch tip (latest commit)
- **FETCH_HEAD, ORIG_HEAD**: Reserved for special purposes
- **refs/heads/**: Branch references
- **refs/tags/**: Tag references
- **Naming rules**: lowercase alphanumeric, hyphens, underscores, dots (no slashes at start/end)
- **Hierarchy**: Use path-like structure for organization (e.g., `refs/heads/feature/login`)

**Takeaway for xs**: Adopt Git's clear HEAD semantics. `head` should consistently mean "most recent" across all contexts.

### NATS Messaging Conventions
**Source**: [NATS Subject Naming - NATS Docs](https://docs.nats.io/nats-concepts/subjects), [NATS Stream/Consumer Naming](https://docs.nats.io/running-a-nats-service/nats_admin/jetstream_admin/naming)

Key principles:
- **Subjects**: Hierarchical with dots as separators (e.g., `accounting.usa.east.orders`)
- **Naming characters**: lowercase alphanumeric, hyphens, underscores (no whitespace, periods in names)
- **System names**: `$` prefix reserved for system use
- **Stream/Consumer names**: Alphanumeric only, underscores allowed, < 32 chars
- **First token establishes namespace**: Broad categories first, specific later

**Takeaway for xs**: Establish clear hierarchical naming with consistent separators (`:` or `_`). System resources could use `xs_` or `_xs` prefix.

### Kafka Best Practices
**Source**: [Kafka Topic Naming - New Relic](https://newrelic.com/blog/best-practices/effective-strategies-kafka-topic-partitioning)

Key principles:
- **Topics**: Logical grouping of messages (analogous to xs "topic")
- **Consumer Groups**: Multiple consumers reading from same topic
- **Naming**: Hierarchical, lowercase, hyphens for word separation
- **Partitioning**: Determines parallelism and consumer scaling

**Takeaway for xs**: Topics should follow consistent hierarchical naming (e.g., `domain.entity.event_type`).

### Redis Naming Conventions
**Source**: [Redis Key Naming - DEV Community](https://dev.to/rijultp/redis-naming-conventions-every-developer-should-know-1ip)

Key principles:
- **Structure**: `environment:service:entity:id:attribute`
- **Separators**: Colons `:` for hierarchy (not periods)
- **Clarity over brevity**: Names should be descriptive
- **Avoid generics**: Don't use `user` or `cache`, be specific
- **Include identifiers**: Make keys unique and traceable
- **Type prefixes**: e.g., `user:123:profile:hash`, `user:123:followers:set`

**Takeaway for xs**: Use colons for hierarchical separation. Include type hints in names. Avoid ambiguous short names.

### Kubernetes Naming
**Source**: [Kubernetes Names and IDs](https://kubernetes.io/docs/concepts/overview/working-with-objects/names/), [Kubernetes Labels](https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/)

Key principles:
- **Names**: 253 char max, lowercase alphanumeric + hyphens + dots
- **Must start/end with alphanumeric**
- **Labels**: Key-value pairs for metadata and filtering
- **Hierarchical structure**: `environment-application-component-version`

**Takeaway for xs**: Adopt similar constraints. Use hyphens for word separation in user-facing names.

---

## Part 3: Proposed Naming Schema

### Core Naming Rules

#### Rule 1: Character Set
- **Internal identifiers (IDs)**: SCRU128 format (existing)
- **Names (topics, contexts, etc.)**:
  - Characters: lowercase `[a-z0-9]`, hyphens `-`, underscores `_`
  - Max length: 253 characters
  - Must start and end with alphanumeric
  - No whitespace, periods, slashes

#### Rule 2: Hierarchical Separator
- **Use**: Colons `:` for semantic hierarchy (following Redis/NATS)
- **Use**: Hyphens `-` for word separation within components
- **Example**: `accounts:user-auth:login-event`

#### Rule 3: Clarity Over Brevity
- Names should be self-documenting
- Avoid ambiguous abbreviations
- Use consistent terminology across codebase

#### Rule 4: Type Hints
- Optional: Include type/kind in naming for complex entities
- Example: `config:database:host`, `metric:request:latency`

---

### Concept Definitions & Naming

#### 1. **Frame** (the basic unit)
- **Definition**: A single event/record in the stream, immutable, with unique ID and timestamp
- **Naming**: Use term "frame" consistently (not "event", "message", "record")
- **Examples**:
  - API: `POST /append/{topic}` → creates a frame
  - CLI: `xs append topic < data.json`
  - Internal: `Frame { id, topic, context_id, hash, meta, ttl }`

#### 2. **Stream** (ordered append-only log)
- **Definition**: Immutable, append-only sequence of frames, one per topic per context
- **Naming**: Use "stream" for the logical concept
- **Note**: Internally stored in partitions, externally referred to as stream
- **Examples**:
  - "Read from the stream" = read frames in order
  - "Subscribe to the stream" = watch for new frames

#### 3. **Topic** (subject/category)
- **Definition**: Subject line for organizing frames; multiple frames can have same topic
- **Naming**: `topic` (not "subject", "category", "stream-name")
- **Format**: `domain:entity:event-type` (hierarchical)
- **Examples**:
  - `accounts:user-auth:login-attempt`
  - `systems:health:cpu-spike`
  - `orders:payment:failed`
- **Rules**:
  - Must follow character set rules
  - Treat as case-sensitive
  - Suggest lowercase with hyphens for readability

#### 4. **Context** (isolation boundary)
- **Definition**: Namespace/scope isolating frames within a topic
- **Naming**: `context` or `context-id` (for the identifier)
- **Important distinction**:
  - "Context ID" = the actual identifier (SCRU128)
  - "Context" = the scope/namespace concept
  - "System context" / "ZERO_CONTEXT" = special context for system operations
- **Examples**:
  - `context-id: 0c8z7k...` (in code/API)
  - `--context user-123` (in CLI)
  - "Create a new context" (in docs)

#### 5. **Index** (lookup mechanism)
- **Definition**: Internal index structure for efficient lookups
- **Naming**: Never use "index" in user-facing APIs
- **Internal naming**: `idx_context`, `idx_topic` (prefix convention)
- **Note**: Indices are implementation detail, not exposed to users

#### 6. **Position/Offset** (location in stream)
- **Definition**: Location of a frame within a stream
- **Naming**:
  - When referring to specific frame: use `id` (not `offset`, `position`, `index`)
  - When describing reading mode: use `from-latest`, `from-beginning`, etc.
- **Examples**:
  - `--from-id 0c8z7k...` (resume from specific frame)
  - `xs cat --from-beginning` (start from oldest)
  - `xs cat --from-latest` (skip to newest)

---

### Operation Naming

#### Reading Operations

| Operation | Current | Recommended | Semantics |
|-----------|---------|-------------|-----------|
| Stream all frames | `.cat` | `.cat` | ✅ Keep (standard Unix name) |
| Get latest | `.head` | `.head` | ✅ Keep (matches Git) |
| Get specific | `.get` | `.get` | ✅ Keep (by ID) |
| Resume from point | `--last-id` | `--from-id` | ⚠️ Rename (clearer semantics) |
| Start from end | `--tail` | `--from-latest` | ⚠️ Rename (less confusing) |
| Start from beginning | (missing) | `--from-beginning` | ✨ Add |
| Watch for updates | `--follow` | `--follow` / `--subscribe` | ✅ Keep |

#### Writing Operations

| Operation | Current | Recommended | Notes |
|-----------|---------|-------------|-------|
| Add to stream | `.append` | `.append` | ✅ Keep |
| Remove frame | `.remove` | `.remove` | ✅ Keep |
| Store content | `.cas` | `.cas` (content-addressable-storage) | ✅ Keep |

#### Query Parameters

| Parameter | Current | Recommended | Notes |
|-----------|---------|-------------|-------|
| Topic filter | `topic` | `--topic` / `-T` | ✅ Standardize on `topic` |
| Context filter | `context` | `--context` / `-c` | ✅ Keep |
| Result limit | `limit` | `--limit` | ✅ Keep |
| Heartbeat interval | `pulse` | `--pulse` / `-p` | ✅ Keep |
| Read all contexts | `--all` / `-a` | `--all` | ✅ Keep |

---

### Command Structure Consistency

#### CLI Command Layer
```
xs COMMAND ARGS [OPTIONS]

xs cat [ADDRESS] [OPTIONS]
  --from-id ID          Resume from specific frame
  --from-latest         Skip existing, show new
  --from-beginning      Start from oldest
  --follow              Subscribe to updates
  --limit N             Max frames to return
  --topic PATTERN       Filter by topic
  --context ID          Context to read from
  --all                 Read across all contexts

xs append [ADDRESS] TOPIC [OPTIONS] < data
  --meta JSON           Frame metadata
  --ttl SPEC            Time-to-live
  --context ID          Context for frame

xs head [ADDRESS] TOPIC [OPTIONS]
  --follow              Watch for updates
  --context ID          Context to query
```

#### API Route Layer
```
GET  /                           # cat (read frames)
  ?from-id=ID&from-latest&topic=T&context-id=C

POST /append/{topic}             # append
  ?context=ID&ttl=SPEC
  Header: xs-meta: base64(json)

GET  /head/{topic}               # head
  ?context-id=C&follow=true

GET  /frame/{id}                 # get frame by ID

DELETE /frame/{id}               # remove frame

GET  /cas/{integrity}            # content-addressable-storage

POST /cas                        # store content

GET  /version                    # server version
```

#### Internal Code Layer
```rust
// Core store operations
pub fn append(&self, frame: Frame) -> Result<Frame, Error>
pub fn head(&self, topic: &str, context_id: Id) -> Option<Frame>
pub fn get(&self, frame_id: Id) -> Option<Frame>
pub fn remove(&self, frame_id: Id) -> Result<(), Error>
pub fn read(&self, options: ReadOptions) -> FrameStream

// ReadOptions structure
pub struct ReadOptions {
    pub from_id: Option<Id>,           // Resume from specific frame
    pub from_latest: bool,             // Skip existing frames
    pub follow: FollowOption,          // Subscribe mode
    pub limit: Option<usize>,          // Limit results
    pub topic: Option<String>,         // Topic filter
    pub context_id: Option<Id>,        // Context filter
}

// FollowOption enum
pub enum FollowOption {
    Off,
    On,
    WithHeartbeat(Duration),
}
```

---

### Boolean Flags and Options

#### Current Confusing Flags
```
--tail                # CONFUSING: Actually means "skip existing" (like `tail -f`)
                      # But description says "begin long after end"
```

#### Recommended Replacement
```
--from-latest         # CLEAR: Start reading from the latest frame
--from-beginning      # CLEAR: Start reading from the oldest frame
--from-id <ID>        # CLEAR: Resume reading from specific frame
--follow              # CLEAR: Keep reading new frames (subscribe mode)
```

#### Semantic Clarity
- **`--from-latest`**: Skip all existing frames, show only new ones (replaces `--tail`)
- **`--from-beginning`**: Include all frames from oldest (default when not specified)
- **`--from-id <ID>`**: Resume from specified frame ID (replaces `--last-id`)
- **`--follow`**: Keep subscription open, receive heartbeats if enabled

---

## Part 4: Special Cases and Edge Cases

### Context Naming
```
ZERO_CONTEXT        # System context (all bits 0), for system operations
"system"            # Suggested alias for ZERO_CONTEXT in user-facing APIs
<user-id>          # Per-user context isolation
<job-id>           # Per-job context isolation
```

### Topic Naming Patterns
```
# Good examples:
accounts:user-auth:login-success
accounts:user-auth:login-failed
payments:transaction:completed
payments:transaction:failed
systems:health:cpu-alert
systems:health:memory-alert

# Anti-patterns to avoid:
user_events         # Too vague
login               # Not specific enough
xs_internal_frame   # Underscore mixing (avoid `_`)
Topic1              # CamelCase not recommended
```

### Reserved Terms
```
# Do NOT use these as topic/context names (or use with caution):
$xs               # System namespace (reserved)
HEAD              # Reserved (special ref)
head              # Built-in operation
all               # Reserved flag
...               # More as determined
```

### TTL Specification
```
# Keep existing TTL format (already clear):
--ttl forever                    # Never expires
--ttl ephemeral                  # Until server restart
--ttl time:5000                  # 5000 milliseconds
--ttl head:10                    # Keep only last 10 per context
```

---

## Part 5: Migration Guide

### Phase 1: Update Core Data Structures (Internal)

**File**: `/src/store/mod.rs`

```rust
// BEFORE
pub struct ReadOptions {
    pub follow: FollowOption,
    pub tail: bool,
    pub last_id: Option<Scru128Id>,
    pub limit: Option<usize>,
    pub context_id: Option<Scru128Id>,
    pub topic: Option<String>,
}

// AFTER
pub struct ReadOptions {
    pub follow: FollowOption,
    pub from_latest: bool,           // Renamed from `tail`
    pub from_id: Option<Scru128Id>, // Renamed from `last_id`
    pub limit: Option<usize>,
    pub context_id: Option<Scru128Id>,
    pub topic: Option<String>,
}
```

**Rationale**: More explicit semantics reduce confusion.

### Phase 2: Update Query Parameter Parsing

**File**: `/src/store/mod.rs` (ReadOptions deserialization)

Keep backward compatibility:
```rust
impl<'de> Deserialize<'de> for ReadOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        // Accept BOTH old and new parameter names:
        // - Old: `tail`, `last-id`
        // - New: `from-latest`, `from-id`
        //
        // Parse old names but internally store as new fields
        // Log deprecation warnings for old names
    }
}

impl ReadOptions {
    pub fn to_query_string(&self) -> String {
        // Generate new parameter names
        // but continue accepting old ones for compatibility
    }
}
```

### Phase 3: Update CLI Layer

**File**: `/src/main.rs`

```rust
// BEFORE
#[derive(Parser, Debug)]
struct CommandCat {
    #[clap(long, short = 't')]
    tail: bool,

    #[clap(long, short = 'l')]
    last_id: Option<String>,
}

// AFTER
#[derive(Parser, Debug)]
struct CommandCat {
    #[clap(long, short = 'f')]
    follow: bool,

    #[clap(long)]
    from_latest: bool,

    #[clap(long)]
    from_beginning: bool,

    #[clap(long)]
    from_id: Option<String>,

    #[clap(long)]
    limit: Option<u64>,

    #[clap(long, short = 'c')]
    context: Option<String>,

    #[clap(long, short = 'a')]
    all: bool,

    #[clap(long = "topic", short = 'T')]
    topic: Option<String>,
}
```

Keep old flags with deprecation warnings for one release:
```rust
#[clap(long, short = 't', hide = true)]  // Hidden but still works
tail: bool,

#[clap(long, short = 'l', hide = true)]
last_id: Option<String>,
```

### Phase 4: Update API Routes

**File**: `/src/api.rs`

Query parameters should support both old and new names during transition:
```rust
// Accept both:
GET /?tail=true                    // deprecated
GET /?from-latest=true            // new, preferred

GET /?last-id=...                 // deprecated
GET /?from-id=...                 // new, preferred

GET /?context-id=...              // keep as-is (unambiguous)
GET /?topic=...                   // keep as-is
GET /?limit=...                   // keep as-is
```

### Phase 5: Update Documentation & Examples

Update all docs, examples, and tests to use new naming:
- README.md
- docs/ directory
- examples/
- API documentation
- CHANGELOG

### Phase 6: Update Internal Code

Update all internal function names and variables:
- Rename `last_id` → `from_id`
- Rename `tail` → `from_latest`
- Update comments and docstrings
- Update variable names in handlers, generators, etc.

### Deprecation Timeline

**v0.X.0**: Introduce new parameter names, accept both old and new (with warnings)
**v0.Y.0**: Remove old parameter names, require migration
**Future**: Major version bump for breaking changes

---

## Part 6: Implementation Checklist

### Documentation Changes
- [ ] Update README.md with new terminology
- [ ] Update docs/reference with `from-latest`, `from-id`, etc.
- [ ] Update all code examples
- [ ] Add migration guide to CHANGELOG
- [ ] Update API documentation

### Code Changes
- [ ] Update `ReadOptions` struct
- [ ] Update query parameter deserialization (with backward compat)
- [ ] Update CLI argument parsing
- [ ] Update API route handling
- [ ] Update all internal usages
- [ ] Update tests and test fixtures
- [ ] Update examples

### Testing
- [ ] Add tests for backward compatibility
- [ ] Test old parameter names produce warnings
- [ ] Test new parameter names work
- [ ] Test default behaviors
- [ ] Integration tests for full flows

### Communication
- [ ] Announce deprecation in release notes
- [ ] Blog post explaining naming improvements
- [ ] Discord announcement
- [ ] Update contributing guidelines

---

## Part 7: Reference Tables

### Concept Summary

| Concept | Definition | Examples | Related Terms |
|---------|-----------|----------|--------------|
| **Frame** | Single immutable event/record | Frame { id, topic, context_id, hash, meta } | Event, message, record, entry |
| **Stream** | Append-only sequence of frames | topic-level stream, context-level stream | Log, journal, queue |
| **Topic** | Subject/category organizing frames | `accounts:auth:login` | Channel, namespace, subject |
| **Context** | Isolation boundary / namespace | ZERO_CONTEXT, user-123, job-456 | Scope, boundary, partition |
| **Index** | Lookup mechanism (internal) | idx_context, idx_topic | Partition, mapping |
| **ID** | Unique identifier | Scru128 format | UUID, identifier, key |

### Operation Summary

| Operation | Meaning | Example | Returns |
|-----------|---------|---------|---------|
| **append** | Add frame to stream | `xs append addr topic` | Frame |
| **cat** | Read frames | `xs cat addr --from-beginning` | FrameStream |
| **head** | Get most recent | `xs head addr topic` | Frame |
| **get** | Get by ID | `xs get addr frame-id` | Frame |
| **remove** | Delete frame | `xs remove addr frame-id` | () |
| **follow** | Subscribe to updates | `--follow` | FrameStream |

### Parameter Summary

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| **from-id** | String (ID) | Resume from frame | `--from-id 0c8z7k...` |
| **from-latest** | Bool | Skip existing frames | `--from-latest` |
| **from-beginning** | Bool | Include all frames | `--from-beginning` |
| **follow** | Bool/Duration | Subscribe mode | `--follow` or `-f` |
| **limit** | Number | Max results | `--limit 10` |
| **topic** | String | Topic filter | `--topic accounts:*` |
| **context** | String (ID) | Context scope | `--context user-123` |
| **all** | Bool | Read all contexts | `--all` or `-a` |

---

## Part 8: FAQ and Rationale

### Q: Why "frame" instead of "event" or "message"?
**A**: "Frame" better captures the movie-like sequence. "Event" implies causality/time in ways that don't apply. "Message" suggests communication, which xs is not primarily about.

### Q: Why not use `last` instead of `head`?
**A**: Git uses `HEAD` to mean "current position." To align with industry standards and reduce confusion, `head` is clearer. However, discussions are ongoing about whether `last` might be better. This guide recommends `head` for consistency with Git, but a future decision could change this.

### Q: Why colons `:` for hierarchy, not periods `.`?
**A**:
- Redis uses colons and they're proven effective
- NATS uses periods but for email-like subjects
- Colons avoid confusion with domain names
- Colons are clearer separators in CLIs

### Q: Why `from-latest` and `--from-id` instead of shorter names?
**A**:
- Clarity over brevity (stated principle)
- Easier to understand at a glance
- Self-documenting in scripts/configs
- Reduces cognitive load for users

### Q: Can we still use `tail` in the codebase?
**A**: Not recommended. Use `from-latest` for new code. Can keep `tail` internally with a comment explaining it means "latest" not "end".

### Q: How do we handle context in queries?
**A**: Always explicit. If `--context` not specified, default to system context (ZERO_CONTEXT). For cross-context queries, use `--all`.

### Q: Should topic names be case-sensitive?
**A**: Yes. Treat as case-sensitive in storage and comparison. Recommend lowercase in user-facing naming conventions.

---

## Part 9: Alignment with Shastra Ecosystem

This naming schema should align with sister projects in the shastra ecosystem:
- [ ] Compare with sibling project conventions
- [ ] Ensure cross-project consistency
- [ ] Coordinate on hierarchical separators
- [ ] Align on context/scope terminology

(Recommendations for specific alignment details should be added as sister projects are reviewed)

---

## References

### Industry Standards
- [Git References - Baeldung](https://www.baeldung.com/ops/git-refs-branch-slash-name)
- [Git Refs and Reflog - Atlassian](https://www.atlassian.com/git/tutorials/refs-and-the-reflog)
- [NATS Subjects - NATS Docs](https://docs.nats.io/nats-concepts/subjects)
- [NATS Naming - NATS Docs](https://docs.nats.io/running-a-nats-service/nats_admin/jetstream_admin/naming)
- [Kafka Topics - New Relic](https://newrelic.com/blog/best-practices/effective-strategies-kafka-topic-partitioning)
- [Redis Naming - DEV Community](https://dev.to/rijultp/redis-naming-conventions-every-developer-should-know-1ip)
- [Kubernetes Names - Kubernetes Docs](https://kubernetes.io/docs/concepts/overview/working-with-objects/names/)
- [Kubernetes Labels - Kubernetes Docs](https://kubernetes.io/docs/concepts/overview/working-with-objects/common-labels/)

### Project References
- xs README.md
- xs CLAUDE.md
- xs source code (src/)
- xs Discord discussion (January 2026)

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-12 | Initial comprehensive naming schema |

---

**Status**: Draft - Awaiting Community Review and Approval

For questions or suggestions, please open an issue or discuss in the [xs Discord](https://discord.com/invite/YNbScHBHrh).
