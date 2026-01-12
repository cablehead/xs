# XS Naming Schema - Visual Reference Guide

*Visual diagrams and decision trees for quick understanding of naming conventions*

---

## 1. Data Model Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     XS EVENT STREAMING STORE                │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │               Global Stream (entire store)            │  │
│  │  ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐  │  │
│  │  │ F │ F │ F │ F │ F │ F │ F │ F │ F │ F │ F │ F │  │  │
│  │  └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘  │  │
│  │   ↑                                           ↑  ← Head │  │
│  │   └─── Oldest (first born)        Newest (most recent)  │  │
│  └──────────────────────────────────────────────────────┘  │
│            (One Frame = One immutable event)                │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    Topic Index: Organize frames by subject            │  │
│  │                                                        │  │
│  │  Topic: accounts:auth:login                          │  │
│  │  ├─ Frame in context "user-123"                     │  │
│  │  ├─ Frame in context "user-456"                     │  │
│  │  └─ Frame in context "system"                       │  │
│  │                                                        │  │
│  │  Topic: payments:transaction:completed               │  │
│  │  ├─ Frame in context "job-789"                      │  │
│  │  └─ Frame in context "system"                       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │    Context Index: Isolation boundaries               │  │
│  │                                                        │  │
│  │  Context: user-123                                   │  │
│  │  ├─ Frames from topic accounts:auth:*               │  │
│  │  ├─ Frames from topic payments:*                    │  │
│  │  └─ Frames from topic systems:*                     │  │
│  │                                                        │  │
│  │  Context: system (ZERO_CONTEXT)                      │  │
│  │  ├─ System frames from all topics                    │  │
│  │  └─ Background job outputs                           │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Frame Structure

```
┌────────────────────────────────────────────┐
│              Frame Object                  │
├────────────────────────────────────────────┤
│ id: Scru128Id                              │ ← Unique identifier
│ topic: "accounts:user-auth:login"          │ ← Subject/category
│ context_id: "user-123"                     │ ← Isolation scope
│ hash: ssri::Integrity                      │ ← Content-addressable
│ meta: {"user": "bob", "ip": "1.2.3.4"}    │ ← Optional metadata
│ ttl: 5000ms | forever | ephemeral | head:N│ ← Retention policy
│ timestamp: implicit in ID                  │ ← From SCRU128
└────────────────────────────────────────────┘
```

---

## 3. CLI Command Taxonomy

```
┌─────────────────────────────────────────────────────────┐
│                  xs COMMAND STRUCTURE                   │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  Reading Commands                                        │
│  ├─ xs cat [address]                                   │
│  │  │  Read/concatenate frames                         │
│  │  └─ Options: --from-latest, --from-beginning,      │
│  │            --from-id, --follow, --limit             │
│  │                                                       │
│  ├─ xs head [address] [topic]                          │
│  │  │  Get most recent frame in topic                  │
│  │  └─ Options: --follow, --context                    │
│  │                                                       │
│  └─ xs get [address] [frame-id]                        │
│     │  Get specific frame by ID                        │
│     └─ No options                                       │
│                                                           │
│  Writing Commands                                        │
│  ├─ xs append [address] [topic]                        │
│  │  │  Add frame to stream                             │
│  │  └─ Options: --meta, --ttl, --context               │
│  │                                                       │
│  └─ xs remove [address] [frame-id]                     │
│     │  Delete frame                                    │
│     └─ No options                                       │
│                                                           │
│  Content Commands                                        │
│  ├─ xs cas [address] [hash]                            │
│  │  │  Get content by hash (content-addressable)       │
│  │  └─ No options                                       │
│  │                                                       │
│  └─ xs cas-post [address]                              │
│     │  Store content                                   │
│     └─ Reads from stdin                                │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

---

## 4. Reading Strategies - Decision Tree

```
                    Want to read frames?
                           │
                ┌──────────┴──────────┐
                │                     │
           All existing?          All new?
           Yes↓          No↑  Yes↓          No↑
                │          │      │          │
         ┌──────────┐   ┌─────────────┐
         │ Use:     │   │ Use:        │
         │ --from-  │   │ --from-     │
         │beginning │   │latest       │
         │          │   │             │
         │Default   │   │ Skip existing│
         │if no flag│   │ Show new only│
         └──────────┘   └─────────────┘
                │
                │
         Resume from specific?
                │
         Yes↓          No↑
         ┌─────────────────┐
         │ Use:            │
         │ --from-id ABC123│
         │                 │
         │ Pick up where   │
         │ you left off    │
         └─────────────────┘

         Keep watching?
                │
         Yes↓          No↑
         ┌─────────────┐
         │ Add:        │
         │ --follow    │
         │ (subscribe) │
         └─────────────┘
```

---

## 5. Operation Matrix

```
┌──────────────┬──────────────┬──────────────┬──────────────┐
│ Operation    │ CLI Example  │ API Route    │ Returns      │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Get Latest   │ xs head addr │ GET /head/   │ Latest Frame │
│              │ topic        │ {topic}      │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Read All     │ xs cat addr  │ GET /?from-  │ Frame Stream │
│              │ --from-      │ beginning=   │              │
│              │ beginning    │ true         │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Read New     │ xs cat addr  │ GET /?from-  │ Frame Stream │
│ Only         │ --from-      │ latest=true  │              │
│              │ latest       │              │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Resume       │ xs cat addr  │ GET /?from-  │ Frame Stream │
│              │ --from-id    │ id=ABC123    │              │
│              │ ABC123       │              │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Add Frame    │ xs append    │ POST         │ Frame ID     │
│              │ addr topic   │ /append/     │              │
│              │              │ {topic}      │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Get Frame    │ xs get addr  │ GET /frame/  │ Frame        │
│              │ ABC123       │ {id}         │              │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ Delete Frame │ xs remove    │ DELETE       │ Success      │
│              │ addr ABC123  │ /frame/{id}  │              │
└──────────────┴──────────────┴──────────────┴──────────────┘
```

---

## 6. Topic Naming Guide

```
Good Topic Names (hierarchical, descriptive)
════════════════════════════════════════════════════════════

Pattern: domain:entity:event-type

Example structure:
┌──────────────────────────────────────────────────────┐
│ accounts:user-auth:login-success                     │
│ ^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^^^^^^                  │
│ Domain   Entity     Event Type                       │
│                                                       │
│ Meaning: User authentication domain → user entity    │
│          → successful login event                    │
└──────────────────────────────────────────────────────┘

More Examples:
═════════════════════════════════════════════════════════

Domain          Entity              Event Type
─────           ──────              ──────────
accounts        user-auth           login-success
accounts        user-auth           login-failed
accounts        user-auth           logout
accounts        user-profile        updated
accounts        user-profile        avatar-changed

payments        transaction         completed
payments        transaction         failed
payments        transaction         refunded
payments        card                linked
payments        card                removed

systems         health              cpu-spike
systems         health              memory-warning
systems         health              disk-full
systems         deployment          started
systems         deployment          completed
systems         deployment          failed


Anti-Patterns (What NOT to do)
═══════════════════════════════════════════════════════════

❌ user_events              # Too vague, missing domain
❌ login                    # Not specific enough
❌ USER_LOGIN              # CamelCase, doesn't scale
❌ accounts/login          # Wrong separator (use :)
❌ xs_internal_frame       # Underscore mixing
❌ accounts:user-auth.login # Period mixing
❌ Topic1                   # Generic numbering
```

---

## 7. Parameter Conversion Guide

```
┌─────────────────────────────────────────────────────────┐
│ LEGACY → NEW PARAMETER MAPPING                          │
├─────────────────────────────────────────────────────────┤
│                                                           │
│ CLI Arguments                                            │
│ ─────────────────────────────────────────────────────   │
│ OLD:  xs cat addr --tail                              │
│ NEW:  xs cat addr --from-latest                       │
│                                                           │
│ OLD:  xs cat addr --last-id abc123                    │
│ NEW:  xs cat addr --from-id abc123                    │
│                                                           │
│ NEW:  xs cat addr --from-beginning                    │
│       (No legacy equivalent - use if no flags)         │
│                                                           │
│                                                           │
│ Query Parameters                                         │
│ ──────────────────────────────────────────────────────  │
│ OLD:  GET /?tail=true                                 │
│ NEW:  GET /?from-latest=true                          │
│                                                           │
│ OLD:  GET /?last-id=abc123                            │
│ NEW:  GET /?from-id=abc123                            │
│                                                           │
│ NEW:  GET /?from-beginning=true                       │
│       (No legacy equivalent)                            │
│                                                           │
│                                                           │
│ Other Parameters (No Change)                            │
│ ──────────────────────────────────────────────────────  │
│ SAME: --follow, -f             # Subscribe mode      │
│ SAME: --limit N                # Max results          │
│ SAME: --topic PATTERN          # Filter by topic      │
│ SAME: --context ID, -c         # Context filter       │
│ SAME: --all, -a                # All contexts         │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

---

## 8. Backward Compatibility Timeline

```
Release Timeline
════════════════════════════════════════════════════════════

v0.X.Y (CURRENT) - Transition Release
├─ ✓ NEW names supported: --from-latest, --from-id
├─ ✓ OLD names still work: --tail, --last-id
├─ ⚠ Deprecation warnings printed
├─ ✓ Full backward compatibility
└─ Timeline: Users can migrate gradually

v0.Y.0 (NEXT) - Breaking Release
├─ ✗ OLD names removed: --tail, --last-id
├─ ✓ NEW names required: --from-latest, --from-id
├─ ✗ No backward compatibility
└─ Timeline: Users must have migrated

───────────────────────────────────────────────────────────

Current   v0.X.Y                    v0.Y.0
Release   (in progress)             (next)
  │           │                       │
  ├─ Old ─────┼─ Old works +           │
  │  names    │   warnings ────→ ✗ Removed
  │  broken   │   Old works           │
  │           │   New works           │
  │           │   (choose one)        │
  │           │                       │
  └─ NEW ─────┼───────────────→ ✓ Required
     names    │
     added    │
```

---

## 9. Concept Hierarchy

```
┌─────────────────────────────────────────────────────┐
│        xs DATA ORGANIZATION HIERARCHY               │
├─────────────────────────────────────────────────────┤
│                                                       │
│  Level 1: GLOBAL STORE                              │
│  └─ One immutable append-only log of ALL frames    │
│                                                       │
│  Level 2: TOPICS (Subject-based organization)       │
│  └─ Organize frames by subject                      │
│     Example: accounts:auth:login                    │
│     │       accounts:auth:logout                    │
│     └─ Multiple topics in store                     │
│                                                       │
│  Level 3: CONTEXTS (Scope-based organization)       │
│  └─ Isolate frames by boundary                      │
│     Example: user-123 (per-user scope)              │
│     │       job-456 (per-job scope)                 │
│     └─ Multiple contexts in store                   │
│                                                       │
│  Level 4: TOPIC-CONTEXT STREAM (Intersection)       │
│  └─ Frames matching BOTH topic AND context          │
│     Example: All "accounts:auth:login" frames       │
│             in context "user-123"                   │
│                                                       │
│  Level 5: FRAMES (Individual events)                │
│  └─ Single immutable event with:                    │
│     - Unique ID (Scru128)                           │
│     - Timestamp (implicit in ID)                    │
│     - Content hash                                  │
│     - Metadata (optional)                           │
│     - TTL (retention policy)                        │
│                                                       │
└─────────────────────────────────────────────────────┘
```

---

## 10. API Endpoint Overview

```
REST API Routes
════════════════════════════════════════════════════════════

GET /                               # Read frames (cat)
    ?from-latest=bool               # Start from latest
    ?from-beginning=bool            # Start from oldest
    ?from-id=ID                     # Resume from point
    ?follow=bool|ms                 # Subscribe mode
    ?limit=N                        # Max results
    ?topic=PATTERN                  # Topic filter
    ?context-id=ID                  # Context filter
    ?all=bool                       # All contexts
    Accept: text/event-stream       # For SSE

HEAD /head/{topic}                  # Get latest frame
    ?context-id=ID                  # Context filter
    ?follow=bool                    # Watch for updates

GET /frame/{id}                     # Get frame by ID

POST /append/{topic}                # Add frame
    ?context=ID                     # Context scope
    ?ttl=SPEC                       # Retention
    xs-meta: base64(json)          # Metadata

DELETE /frame/{id}                  # Remove frame

GET /cas/{hash}                     # Get content by hash

POST /cas                           # Store content

GET /version                        # Server version

GET /eval                           # (Special: Nushell)

POST /import                        # (Special: Bulk import)
```

---

## 11. Quick Decision Chart - "Am I naming this right?"

```
IS IT A SINGLE EVENT?
    │
    ├─ YES → Use "FRAME"
    │        "Add a frame to the stream"
    │        Not: "Add an event", "Add a message"
    │
    └─ NO → Continue...

IS IT AN ORDERED SEQUENCE OF EVENTS?
    │
    ├─ YES → Use "STREAM"
    │        "Read from the stream"
    │        Not: "Read from the queue", "Read from the log"
    │
    └─ NO → Continue...

IS IT A SUBJECT/CATEGORY FOR ORGANIZING?
    │
    ├─ YES → Use "TOPIC"
    │        "Filter by topic"
    │        Format: domain:entity:event-type
    │
    └─ NO → Continue...

IS IT AN ISOLATION BOUNDARY/NAMESPACE?
    │
    ├─ YES → Use "CONTEXT"
    │        "Set context to user-123"
    │        Not: "Set scope to..."
    │
    └─ NO → Continue...

IS IT AN INTERNAL LOOKUP MECHANISM?
    │
    ├─ YES → Use "INDEX" (internal only)
    │        Never expose to users
    │        Not: "Use index to query"
    │
    └─ NO → You might have a new concept!
             Consider documenting it.
```

---

## 12. Common Phrases - Correct vs Incorrect

```
Correct                            │ Incorrect
───────────────────────────────────┼──────────────────────────
"Add a frame to the stream"         │ "Add an event to the queue"
"Read frames from the stream"       │ "Read messages from the log"
"Filter by topic"                   │ "Filter by subject"
"Set the context"                   │ "Set the scope"
"The head of the stream"            │ "The tail of the stream"
"From the latest frame"             │ "From the tail"
"From the beginning"                │ "From the head"
"Resume from frame ID..."           │ "Resume from last-id..."
"Use --from-latest"                 │ "Use --tail"
"Use --from-id"                     │ "Use --last-id"
"The most recent frame"             │ "The newest message"
"An immutable log"                  │ "A mutable stream"
"Topic: accounts:auth:login"        │ "Topic: user_login"
```

---

## Document Links

| For... | Read... |
|--------|---------|
| **Full details** | [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) |
| **Quick lookup** | [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) |
| **Implementation** | [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) |
| **Executive view** | [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md) |
| **Visual guide** | This file (NAMING_VISUAL_REFERENCE.md) |

---

*All diagrams and decision trees are designed to make xs naming conventions intuitive and self-evident.*
