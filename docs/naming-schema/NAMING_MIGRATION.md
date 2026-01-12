# XS Naming Schema - Migration Implementation Guide

*This document provides step-by-step instructions for implementing the new naming schema in the xs codebase.*

---

## Overview

The naming migration consists of 6 phases:
1. Update core data structures
2. Update query parameter parsing
3. Update CLI layer
4. Update API routes
5. Update documentation
6. Update internal code

**Total scope**: ~150 files across 6 categories

---

## Phase 1: Update Core Data Structures

### File: `src/store/mod.rs`

#### Step 1.1: Update `ReadOptions` struct

```rust
// BEFORE
#[derive(PartialEq, Deserialize, Clone, Debug, Default, bon::Builder)]
pub struct ReadOptions {
    #[serde(default)]
    #[builder(default)]
    pub follow: FollowOption,
    #[serde(default, deserialize_with = "deserialize_bool")]
    #[builder(default)]
    pub tail: bool,                    // ← CHANGE THIS
    #[serde(rename = "last-id")]
    pub last_id: Option<Scru128Id>,   // ← CHANGE THIS
    pub limit: Option<usize>,
    #[serde(rename = "context-id")]
    pub context_id: Option<Scru128Id>,
    pub topic: Option<String>,
}

// AFTER
#[derive(PartialEq, Deserialize, Clone, Debug, Default, bon::Builder)]
pub struct ReadOptions {
    #[serde(default)]
    #[builder(default)]
    pub follow: FollowOption,
    #[serde(default, deserialize_with = "deserialize_bool")]
    #[builder(default)]
    pub from_latest: bool,              // ← NEW NAME
    #[serde(rename = "from-id")]
    pub from_id: Option<Scru128Id>,    // ← NEW NAME
    pub limit: Option<usize>,
    #[serde(rename = "context-id")]
    pub context_id: Option<Scru128Id>,
    pub topic: Option<String>,
    // Add new field
    #[serde(default, deserialize_with = "deserialize_bool")]
    #[builder(default)]
    pub from_beginning: bool,           // ← NEW FIELD
}
```

**Why the change**:
- `tail` → `from_latest`: Clearer semantics
- `last_id` → `from_id`: Less ambiguous
- `from_beginning`: New option for completeness

#### Step 1.2: Update serde deserialization for backward compatibility

```rust
impl<'de> Deserialize<'de> for ReadOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Intermediate struct for deserialization
        #[derive(Deserialize)]
        struct ReadOptionsRaw {
            #[serde(default)]
            pub follow: FollowOption,
            #[serde(default)]
            pub tail: Option<bool>,              // Accept old name
            #[serde(default)]
            pub from_latest: Option<bool>,      // Accept new name
            #[serde(default)]
            pub from_beginning: Option<bool>,
            #[serde(rename = "last-id")]
            pub last_id: Option<Scru128Id>,     // Accept old name
            #[serde(rename = "from-id")]
            pub from_id: Option<Scru128Id>,     // Accept new name
            #[serde(rename = "context-id")]
            pub context_id: Option<Scru128Id>,
            pub limit: Option<usize>,
            pub topic: Option<String>,
        }

        let raw = ReadOptionsRaw::deserialize(deserializer)?;

        // Handle backward compatibility
        let from_latest = raw.from_latest.unwrap_or_else(|| {
            if raw.tail.is_some() {
                eprintln!("DEPRECATION WARNING: --tail is deprecated, use --from-latest instead");
                raw.tail.unwrap_or(false)
            } else {
                false
            }
        });

        let from_id = raw.from_id.or(raw.last_id).map(|id| {
            if raw.last_id.is_some() {
                eprintln!("DEPRECATION WARNING: --last-id is deprecated, use --from-id instead");
            }
            id
        });

        Ok(ReadOptions {
            follow: raw.follow,
            from_latest,
            from_id,
            from_beginning: raw.from_beginning.unwrap_or(false),
            limit: raw.limit,
            context_id: raw.context_id,
            topic: raw.topic,
        })
    }
}
```

#### Step 1.3: Update `to_query_string` method

```rust
impl ReadOptions {
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        // Add follow parameter
        match self.follow {
            FollowOption::Off => {}
            FollowOption::On => params.push(("follow", "true".to_string())),
            FollowOption::WithHeartbeat(duration) => {
                params.push(("follow", duration.as_millis().to_string()));
            }
        }

        if let Some(context_id) = self.context_id {
            params.push(("context-id", context_id.to_string()));
        }

        // Use new names in output
        if self.from_latest {
            params.push(("from-latest", "true".to_string()));
        }

        if self.from_beginning {
            params.push(("from-beginning", "true".to_string()));
        }

        if let Some(from_id) = self.from_id {
            params.push(("from-id", from_id.to_string()));
        }

        if let Some(limit) = self.limit {
            params.push(("limit", limit.to_string()));
        }

        if let Some(topic) = &self.topic {
            params.push(("topic", topic.clone()));
        }

        if params.is_empty() {
            String::new()
        } else {
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(params)
                .finish()
        }
    }
}
```

#### Step 1.4: Update any tests in `src/store/mod.rs`

Find all references to `.tail` and `.last_id` in tests, update to `.from_latest` and `.from_id`.

---

## Phase 2: Update Query Parameter Parsing

### File: `src/api.rs`

#### Step 2.1: Update route handling for query parameters

Find where query parameters are parsed and ensure they handle both old and new names:

```rust
fn match_route(
    method: &Method,
    path: &str,
    headers: &hyper::HeaderMap,
    query: Option<&str>,
) -> Routes {
    let params: HashMap<String, String> =
        url::form_urlencoded::parse(query.unwrap_or("").as_bytes())
            .into_owned()
            .collect();

    // The ReadOptions deserialization now handles both old and new names
    // So no changes needed here - it's handled by serde_urlencoded

    match (method, path) {
        // ... rest of routing logic
    }
}
```

**Note**: Since we implemented backward compatibility in `ReadOptions` deserialization, the API layer doesn't need changes. The `serde_urlencoded` crate will automatically use our custom deserializer.

#### Step 2.2: Add deprecation warnings in API responses (optional)

If you want to warn API users about deprecated parameters, add a custom response header:

```rust
// When deprecated parameters are detected, add a header
if raw.tail.is_some() || raw.last_id.is_some() {
    response = response.header(
        "Deprecation",
        "true"
    ).header(
        "Sunset",
        "Wed, 01 Jan 2026 00:00:00 GMT"  // Version when deprecated params will be removed
    );
}
```

---

## Phase 3: Update CLI Layer

### File: `src/main.rs`

#### Step 3.1: Update `CommandCat` struct

```rust
// BEFORE
#[derive(Parser, Debug)]
struct CommandCat {
    #[clap(value_parser)]
    addr: String,

    #[clap(long, short = 'f')]
    follow: bool,

    #[clap(long, short = 'p')]
    pulse: Option<u64>,

    #[clap(long, short = 't')]
    tail: bool,                    // ← DEPRECATED

    #[clap(long, short = 'l')]
    last_id: Option<String>,      // ← DEPRECATED

    #[clap(long)]
    limit: Option<u64>,

    #[clap(long)]
    sse: bool,

    #[clap(long, short = 'c')]
    context: Option<String>,

    #[clap(long, short = 'a')]
    all: bool,

    #[clap(long = "topic", short = 'T')]
    topic: Option<String>,
}

// AFTER
#[derive(Parser, Debug)]
struct CommandCat {
    #[clap(value_parser)]
    addr: String,

    #[clap(long, short = 'f')]
    follow: bool,

    #[clap(long, short = 'p')]
    pulse: Option<u64>,

    // New naming
    #[clap(long)]
    from_latest: bool,                   // ← NEW

    #[clap(long)]
    from_beginning: bool,                // ← NEW

    #[clap(long)]
    from_id: Option<String>,            // ← NEW

    // Keep old names but hide them (for backward compatibility)
    #[clap(long, short = 't', hide = true)]
    tail: bool,                          // ← HIDDEN

    #[clap(long, short = 'l', hide = true)]
    last_id: Option<String>,            // ← HIDDEN

    #[clap(long)]
    limit: Option<u64>,

    #[clap(long)]
    sse: bool,

    #[clap(long, short = 'c')]
    context: Option<String>,

    #[clap(long, short = 'a')]
    all: bool,

    #[clap(long = "topic", short = 'T')]
    topic: Option<String>,
}
```

#### Step 3.2: Update command handler function

```rust
// BEFORE
async fn cat(args: CommandCat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut options = ReadOptions::builder().tail(args.tail).build();
    if let Some(last_id) = args.last_id {
        options.last_id = Some(Scru128Id::from_str(&last_id)?);
    }
    // ...
}

// AFTER
async fn cat(args: CommandCat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Handle both old and new arguments
    let from_latest = args.from_latest || args.tail;
    if args.tail {
        eprintln!("DEPRECATION: --tail is deprecated, use --from-latest instead");
    }

    let from_id = args.from_id
        .or_else(|| {
            if args.last_id.is_some() {
                eprintln!("DEPRECATION: --last-id is deprecated, use --from-id instead");
            }
            args.last_id.clone()
        });

    let mut options = ReadOptions::builder()
        .from_latest(from_latest)
        .from_beginning(args.from_beginning)
        .build();

    if let Some(from_id_str) = from_id {
        options.from_id = Some(Scru128Id::from_str(&from_id_str)?);
    }

    // ... rest of function
}
```

#### Step 3.3: Update help text and documentation

Update the `#[clap(...)]` attributes to reflect new naming:

```rust
#[derive(Parser, Debug)]
struct CommandCat {
    /// Address to connect to [HOST]:PORT or <PATH> for Unix domain socket
    #[clap(value_parser)]
    addr: String,

    /// Follow the stream for new data
    #[clap(long, short = 'f')]
    follow: bool,

    /// Specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
    #[clap(long, short = 'p')]
    pulse: Option<u64>,

    /// Start reading from the latest frame (skip existing data)
    #[clap(long)]
    from_latest: bool,

    /// Include all frames from the beginning (default)
    #[clap(long)]
    from_beginning: bool,

    /// Resume reading from a specific frame ID
    #[clap(long)]
    from_id: Option<String>,

    /// Limit the number of frames to return
    #[clap(long)]
    limit: Option<u64>,

    /// Use Server-Sent Events format
    #[clap(long)]
    sse: bool,

    /// Context ID (defaults to system context)
    #[clap(long, short = 'c')]
    context: Option<String>,

    /// Retrieve all frames, across contexts
    #[clap(long, short = 'a')]
    all: bool,

    /// Filter by topic pattern
    #[clap(long = "topic", short = 'T')]
    topic: Option<String>,

    /// (DEPRECATED: use --from-latest) Skip existing events, only show new ones
    #[clap(long, short = 't', hide = true)]
    tail: bool,

    /// (DEPRECATED: use --from-id) Last event ID to start from
    #[clap(long, short = 'l', hide = true)]
    last_id: Option<String>,
}
```

---

## Phase 4: Update API Routes

### File: `src/api.rs` (Route handling)

#### Step 4.1: Verify API route names

The REST API should use consistent parameter names. Verify:

```rust
// These routes should all support both old and new parameter names:
GET /?from-latest=true              // NEW
GET /?tail=true                     // OLD (deprecated)

GET /?from-id=ID                    // NEW
GET /?last-id=ID                    // OLD (deprecated)

GET /?from-beginning=true           // NEW

GET /?follow=true                   // KEEP
GET /?context-id=ID                 // KEEP
GET /?limit=N                        // KEEP
```

The backward compatibility is already handled in `ReadOptions` deserialization, so no changes needed at the route level.

---

## Phase 5: Update Documentation

### Files to Update:

1. **README.md**
   - Update all examples to use new parameter names
   - Add migration note in "Upgrading" section

2. **docs/** (if exists)
   - Update API reference
   - Update CLI reference
   - Update tutorials and guides

3. **examples/**
   - Update all example files to use new naming
   - Add comments explaining the change

4. **Tests** (all test files)
   - Update all test code to use new naming
   - Add tests for backward compatibility

### Example Changes:

```markdown
// Before
cat /path/to/store --tail --follow

// After
cat /path/to/store --from-latest --follow
```

---

## Phase 6: Update Internal Code

### Scan and Replace Pattern

Search for all occurrences of old naming in the codebase:

```bash
# Find all references
grep -r "\.tail\|last_id\|last-id" src/ --include="*.rs" | grep -v "test" | head -20

# Find in specific files:
grep -r "pub tail\|pub last_id" src/ --include="*.rs"
```

### Code Changes Needed:

1. **Store implementation** (`src/store/mod.rs`)
   - Update variable names
   - Update comments
   - Update internal logic

2. **Handlers** (`src/handlers/`)
   - Update any references to `tail` or `last_id`

3. **Generators** (`src/generators/`)
   - Update any references to `tail` or `last_id`

4. **Nushell integration** (`src/nu/`)
   - Update command implementations
   - Update generated command documentation

5. **Tests** (`src/**/*tests.rs`)
   - Update test fixtures
   - Update assertion messages

6. **Examples** (examples/)
   - Update all example code

---

## Testing Checklist

### Unit Tests
- [ ] `ReadOptions` deserialization with new names
- [ ] `ReadOptions` deserialization with old names (deprecation)
- [ ] `to_query_string()` generates new names
- [ ] All parameters serialize/deserialize correctly

### Integration Tests
- [ ] CLI with `--from-latest`
- [ ] CLI with `--from-beginning`
- [ ] CLI with `--from-id`
- [ ] CLI with deprecated `--tail` (should work with warning)
- [ ] CLI with deprecated `--last-id` (should work with warning)
- [ ] API with `?from-latest=true`
- [ ] API with deprecated `?tail=true`

### Backward Compatibility Tests
- [ ] Old queries still work
- [ ] Old CLI flags still work
- [ ] Deprecation warnings are printed
- [ ] New and old can't be mixed (mutual exclusion)

### Example Code Tests
- [ ] All examples run successfully
- [ ] Examples produce expected output

---

## Release Notes Template

```markdown
## v0.X.Y - Naming Schema Update

### Breaking Changes
- ⚠️ Deprecation: `--tail` and `--last-id` CLI flags will be removed in v0.Y.0
- ⚠️ Deprecation: `tail` and `last-id` query parameters will be removed in v0.Y.0

### New Features
- New CLI flags: `--from-latest`, `--from-beginning`, `--from-id`
- New query parameters: `from-latest`, `from-beginning`, `from-id`

### Deprecations (this release)
- `--tail` is deprecated → use `--from-latest`
- `--last-id` is deprecated → use `--from-id`
- Query parameter `tail` is deprecated → use `from-latest`
- Query parameter `last-id` is deprecated → use `from-id`

### Migration Guide
See [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) for detailed migration steps.

Quick migration:
```bash
# Before
xs cat addr --tail --follow
xs cat addr --last-id abc123

# After
xs cat addr --from-latest --follow
xs cat addr --from-id abc123
```

### Timeline
- v0.X.Y (current): Deprecation warnings, old flags still work
- v0.Y.0 (next major): Remove old flag support, require migration
```

---

## Validation Checklist

Before releasing, verify:

- [ ] All tests pass
- [ ] No compiler warnings
- [ ] Backward compatibility verified
- [ ] Deprecation warnings work
- [ ] Documentation updated
- [ ] Examples updated
- [ ] Release notes prepared
- [ ] Discord announcement drafted
- [ ] CHANGELOG updated

---

## Rollback Plan

If issues are discovered after release:

1. Revert the commit(s)
2. Keep the old naming in the codebase
3. Note the issue in release notes
4. Plan fixes for next release

---

## Post-Implementation Review

After implementation is complete:

- [ ] Gather community feedback
- [ ] Review deprecation warnings (are they helpful?)
- [ ] Check for any missed references
- [ ] Plan final removal in next major version
- [ ] Update NAMING_SCHEMA.md with implementation notes
