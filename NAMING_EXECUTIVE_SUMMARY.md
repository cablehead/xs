# XS Naming Schema - Executive Summary

## Problem Statement

The xs project has inconsistent naming conventions that create confusion for users:

1. **`--tail` semantic confusion**: The flag description says "Begin long after the end of the stream" but users expect it to mean "show the tail end of the stream"
2. **`--last-id` ambiguity**: Unclear if it means "resume from this ID" or "up to but not including this ID"
3. **Terminology overloading**: "context", "index", "stream", "topic", "frame" used inconsistently
4. **CLI/API/Internal inconsistency**: Different naming conventions across layers (`.head`, `HeadGet`, `head()`)

Users asked:
> "If head is the most recent, how can the tail be somewhere long after the end?"

---

## Solution Overview

A comprehensive naming schema that:
- ✅ Aligns with industry standards (Git, NATS, Kafka, Redis, Kubernetes)
- ✅ Eliminates semantic confusion
- ✅ Maintains backward compatibility during transition
- ✅ Provides clear migration path
- ✅ Is well-documented for future maintainers

---

## Key Changes

### 1. CLI Flags (Most User-Visible)

| Problem | Old Name | New Name | Example |
|---------|----------|----------|---------|
| Confusing semantics | `--tail` | `--from-latest` | `xs cat addr --from-latest` |
| Unclear intent | `--last-id` | `--from-id` | `xs cat addr --from-id abc123` |
| Missing option | (none) | `--from-beginning` | `xs cat addr --from-beginning` |

### 2. Core Concepts (Clear Definitions)

- **Frame**: Single immutable event (like Git commit)
- **Stream**: Ordered append-only log of frames
- **Topic**: Subject/category for frames (like Git branch intent)
- **Context**: Isolation boundary/namespace
- **Index**: Internal lookup mechanism (never exposed to users)

### 3. Naming Conventions

**Topics** (hierarchical, user-facing):
```
accounts:user-auth:login-success
payments:transaction:completed
systems:health:cpu-alert
```

**Parameters** (clear, explicit):
```
--from-latest       # Start from newest
--from-beginning    # Include all (from oldest)
--from-id <ID>      # Resume from specific frame
--follow            # Subscribe mode
```

---

## Industry Alignment

### Git
- Uses `HEAD` to mean "current position" (most recent)
- Supports `refs/` hierarchy
- Clear, explicit terminology

→ **xs adopts**: `head` = most recent (aligns with Git, not Unix `tail`)

### NATS
- Hierarchical subjects with dots: `accounting.usa.east.orders`
- Clear consumer/stream semantics
- Explicit parameter naming

→ **xs adopts**: Hierarchical naming with colons: `accounts:user:login`

### Kafka
- Topic = logical grouping of messages
- Consumer groups = multiple readers
- Clear naming conventions

→ **xs adopts**: `topic` = subject (not "stream", not "subject")

### Redis
- Hierarchical keys: `user:123:profile:hash`
- Colons for separator
- Type hints in names

→ **xs adopts**: Colon separator, type hints optional

### Kubernetes
- Clear naming: 253 char max, lowercase + hyphens
- Hierarchical organization
- Explicit semantics

→ **xs adopts**: Character set rules, explicit naming

---

## Migration Approach

### Phase 1: Non-Breaking
- Add new parameter names alongside old ones
- Accept both in CLI and API
- Print deprecation warnings
- **Users can**: Migrate gradually, no forced updates

### Phase 2: Deprecation
- Mark old names as deprecated
- Guide users to new names
- Plan removal in next major version
- **Users must**: Plan for removal

### Phase 3: Removal
- Remove old name support
- Require new naming
- Major version bump

---

## Scope

### What's Included
- [x] Comprehensive naming schema (NAMING_SCHEMA.md)
- [x] Quick reference guide (NAMING_QUICK_REFERENCE.md)
- [x] Migration implementation guide (NAMING_MIGRATION.md)
- [x] Industry research and rationale
- [x] Backward compatibility plan
- [x] Testing strategy

### What's NOT Included (Out of Scope)
- Implementation (code changes) - left for team to execute
- Integration with sister projects (Shastra ecosystem) - to be coordinated separately
- Comprehensive UX testing - will be done post-implementation

---

## Benefits

### For Users
✅ **Clarity**: Parameter names are self-documenting
✅ **Consistency**: Same naming across CLI, API, internal code
✅ **Alignment**: Matches industry standards (Git, NATS, etc.)
✅ **Confidence**: No more confusion about semantics
✅ **Learnability**: New users have easier time understanding concepts

### For Developers
✅ **Maintainability**: Clear terminology across codebase
✅ **Consistency**: Reduces bugs from misnamed variables
✅ **Documentation**: Self-evident parameter meanings
✅ **Extensibility**: Framework for naming new features
✅ **Compatibility**: Gradual migration path

### For Project
✅ **Professionalism**: Industry-standard naming
✅ **Sustainability**: Easier for new contributors
✅ **Growth**: Ready for ecosystem expansion
✅ **Community**: Clearer communication about concepts

---

## Documents Provided

### 1. **NAMING_SCHEMA.md** (Main Document)
- Complete naming reference
- Concept definitions
- Detailed rationale for each choice
- Industry best practices
- Special cases and edge cases
- Migration guide
- Implementation checklist

**Read this for**: Understanding the full schema and rationale

### 2. **NAMING_QUICK_REFERENCE.md** (Cheat Sheet)
- At-a-glance summary of changes
- Common patterns and examples
- CLI quick reference
- API parameter mapping
- Deprecation status

**Read this for**: Quick lookup while implementing or using xs

### 3. **NAMING_MIGRATION.md** (Implementation Guide)
- Step-by-step code changes
- File-by-file instructions
- Code examples for each change
- Testing checklist
- Release notes template
- Validation checklist

**Read this for**: Implementing the schema in the codebase

### 4. **NAMING_EXECUTIVE_SUMMARY.md** (This Document)
- Problem statement
- Solution overview
- Key changes
- Industry alignment
- Benefits and scope

**Read this for**: High-level understanding and decision-making

---

## Recommendations for Next Steps

### Immediate (Before Implementation)
1. **Review as a team**: Read NAMING_SCHEMA.md, discuss in Discord
2. **Get feedback**: Ask community about proposed changes
3. **Make decisions**: Confirm team is aligned on approach
4. **Plan timeline**: Decide which version will include deprecation

### Short-term (Implementation)
1. **Assign ownership**: Who implements which phase?
2. **Create branches**: Start implementing phase by phase
3. **Code review**: Ensure consistent implementation
4. **Test thoroughly**: Especially backward compatibility
5. **Release**: Include release notes and migration guide

### Medium-term (Post-Release)
1. **Monitor feedback**: Watch for issues or confusion
2. **Support migration**: Help users update their scripts/code
3. **Plan removal**: When will old names be removed?
4. **Document lessons**: Update guides based on feedback

---

## FAQ

**Q: Why not use `last` and `first` instead of `head` and `tail`?**
A: Git uses `HEAD` which already means "current position". To align with the most widely-known version control system, we use `head`. However, `last` and `first` were also considered viable.

**Q: Will old parameter names still work after the update?**
A: Yes, in the first release they'll work with deprecation warnings. They'll be removed in a future major version, giving users time to migrate.

**Q: Does this affect the Shastra ecosystem?**
A: Not directly, but sister projects should be notified of naming conventions for consistency. Cross-ecosystem alignment should be coordinated separately.

**Q: What about user-defined topic names? Are they affected?**
A: Not affected. Users can use any characters allowed by the naming rules. The schema recommends best practices but doesn't enforce them.

**Q: Is this a breaking change?**
A: No. The first release maintains backward compatibility while introducing new naming. Breaking changes come in a future major version.

---

## Success Criteria

✅ All major concepts enumerated and clearly defined
✅ Industry best practices researched and documented
✅ Naming schema is clear, consistent, and well-reasoned
✅ Schema addresses the current confusions (head/tail, inclusive/exclusive, etc.)
✅ Documentation is comprehensive enough for implementation
✅ Backward compatibility path is clear
✅ Migration guide is step-by-step and actionable
✅ Community can understand and provide feedback

---

## Contact & Feedback

For questions, suggestions, or feedback on the naming schema:
- **Discord**: [xs Discord](https://discord.com/invite/YNbScHBHrh)
- **GitHub**: Open an issue
- **Docs**: Full details in NAMING_SCHEMA.md

---

## Document References

- **Full Schema**: [NAMING_SCHEMA.md](./NAMING_SCHEMA.md)
- **Quick Reference**: [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md)
- **Migration Guide**: [NAMING_MIGRATION.md](./NAMING_MIGRATION.md)

---

**Version**: 1.0 | **Date**: 2026-01-12 | **Status**: Draft - Ready for Community Review

*This document and accompanying guides were created to establish consistent, industry-aligned naming conventions for the xs project. All major concepts, industry best practices, and migration steps are comprehensively documented.*
