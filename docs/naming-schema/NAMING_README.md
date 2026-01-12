# XS Naming Schema Documentation

Complete naming convention guide for the xs project, establishing consistent, industry-aligned terminology across all components.

## 📚 Documentation Structure

### 🎯 **Start Here: Overview & Summary**

#### [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md) *← Start here*
High-level overview of the problem, solution, and recommendations.
- Problem statement
- Solution overview
- Key changes at a glance
- Industry alignment
- Benefits and scope
- **Read this for**: Decision-making and understanding the big picture

---

### 📖 **Main Reference Documents**

#### [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) *← Comprehensive guide*
Complete, detailed naming schema with full rationale and migration plan.
- Current state analysis with enumerated concepts
- Industry best practices research
- Proposed naming schema with definitions
- Special cases and edge cases
- Migration guide (6 phases)
- Implementation checklist
- FAQ and rationale
- **Read this for**: Understanding every detail and making implementation decisions

#### [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) *← Cheat sheet*
Quick lookup guide for naming conventions, CLI examples, and common patterns.
- At-a-glance summary of changes
- Core concepts in brief
- CLI cheat sheet
- API parameter mapping
- Naming rules
- Common patterns
- Deprecation status
- **Read this for**: Quick lookup while implementing or using xs

#### [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md) *← Diagrams & charts*
Visual diagrams, decision trees, and reference charts.
- Data model overview diagram
- Frame structure
- CLI command taxonomy
- Reading strategies (decision tree)
- Operation matrix
- Topic naming guide
- Parameter conversion guide
- Backward compatibility timeline
- Concept hierarchy
- API endpoint overview
- Decision chart
- Common phrases (correct vs incorrect)
- **Read this for**: Visual understanding and quick reference while working

---

### 🔨 **Implementation Guide**

#### [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) *← Implementation steps*
Step-by-step code changes for implementing the new naming schema.
- 6 phases of implementation
- File-by-file instructions with code examples
- Backward compatibility implementation details
- Testing checklist
- Release notes template
- Validation checklist
- Rollback plan
- **Read this for**: Actually implementing the schema in the codebase

---

## 🗺️ Reading Guide by Role

### 👥 **For Project Maintainers/Decision Makers**
1. Start with [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md)
2. Review [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) Part 3 (Proposed Schema)
3. Check [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) for scope and effort

### 👨‍💻 **For Developers Implementing the Schema**
1. Read [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) Part 3 & 4 (Schema & Migration)
2. Follow [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) step-by-step
3. Use [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) for quick lookups
4. Check [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md) for examples

### 📚 **For Users Learning xs**
1. Review [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md)
2. Check [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md) for diagrams
3. Reference [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) Part 3 for definitions

### 📝 **For Documentation Writers**
1. Read [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) for comprehensive understanding
2. Use [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) as user guide template
3. Check [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md) for examples and diagrams

---

## 🎯 Key Takeaways

### The Main Changes
```
Parameter      Old Name    New Name              Reason
─────────────────────────────────────────────────────────
Skip to latest --tail      --from-latest         Clearer semantics
Resume point   --last-id   --from-id             More explicit
Start from old (missing)   --from-beginning      Consistency
```

### Core Concepts
- **Frame**: Single immutable event
- **Stream**: Ordered append-only log of frames
- **Topic**: Subject/category (hierarchical: `domain:entity:event`)
- **Context**: Isolation boundary/namespace
- **Index**: Internal lookup (never exposed to users)

### Naming Principles
1. **Clarity over brevity**: Names should be self-documenting
2. **Industry alignment**: Follow Git, NATS, Kafka, Redis conventions
3. **Hierarchical structure**: Use colons `:` for hierarchy
4. **Explicit semantics**: Parameters should be unambiguous

### Backward Compatibility
- Old parameters still work (with deprecation warnings)
- Gradual migration path over multiple releases
- Full details in [NAMING_MIGRATION.md](./NAMING_MIGRATION.md)

---

## 📋 Quick Navigation

| What I need... | Read... |
|---|---|
| High-level overview | [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md) |
| Complete reference | [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) |
| Quick cheat sheet | [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) |
| Visual diagrams | [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md) |
| Implementation steps | [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) |
| This index | [NAMING_README.md](./NAMING_README.md) (you are here) |

---

## ✅ Document Checklist

- [x] NAMING_EXECUTIVE_SUMMARY.md - High-level overview
- [x] NAMING_SCHEMA.md - Comprehensive guide (9 parts)
- [x] NAMING_QUICK_REFERENCE.md - Quick lookup
- [x] NAMING_VISUAL_REFERENCE.md - Diagrams & charts
- [x] NAMING_MIGRATION.md - Implementation guide (6 phases)
- [x] NAMING_README.md - This index (you are here)

---

## 🚀 Getting Started

### Step 1: Understand the Problem (5 min)
Read the first section of [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md)

### Step 2: Review the Solution (15 min)
Skim [NAMING_QUICK_REFERENCE.md](./NAMING_QUICK_REFERENCE.md) to see the changes

### Step 3: Deep Dive (30 min)
Read [NAMING_SCHEMA.md](./NAMING_SCHEMA.md) Part 3 for complete schema

### Step 4: Plan Implementation (20 min)
Review [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) phases to understand scope

### Step 5: Implement (4-6 hours)
Follow [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) step-by-step with code examples

---

## ❓ FAQ

**Q: Where do I start?**
A: Start with [NAMING_EXECUTIVE_SUMMARY.md](./NAMING_EXECUTIVE_SUMMARY.md), then move to the document appropriate for your role (see Reading Guide above).

**Q: What's the difference between all these documents?**
A: Each serves a different purpose:
- Executive Summary: Quick overview for decision-makers
- Schema: Complete reference with rationale
- Quick Reference: Cheat sheet for daily use
- Visual Reference: Diagrams and decision trees
- Migration: Step-by-step implementation guide
- This file: Navigation and index

**Q: Do I need to read all of them?**
A: No. Read based on your role (see Reading Guide). Most people only need 2-3 documents.

**Q: When will this be implemented?**
A: Timeline depends on team decision. See [NAMING_MIGRATION.md](./NAMING_MIGRATION.md) for implementation scope.

**Q: Will old parameter names still work?**
A: Yes, during transition phase. See "Backward Compatibility Timeline" in [NAMING_VISUAL_REFERENCE.md](./NAMING_VISUAL_REFERENCE.md).

**Q: How do I provide feedback?**
A: Discuss in [xs Discord](https://discord.com/invite/YNbScHBHrh) or open a GitHub issue.

---

## 📞 Support & Feedback

- **Discord**: [xs Discord Community](https://discord.com/invite/YNbScHBHrh)
- **Issues**: GitHub Issues on the xs repository
- **Documentation**: Full details in the guide documents

---

## 📄 Document Metadata

- **Created**: 2026-01-12
- **Status**: Draft - Ready for Community Review
- **Version**: 1.0
- **Scope**: Complete naming schema for xs project
- **Total Pages**: ~40 pages across 6 documents

---

## 🎓 Learning Path

```
Start
  │
  ├─→ NAMING_EXECUTIVE_SUMMARY (5 min)
  │      ↓
  │   Understand problem & solution
  │      ↓
  ├─→ NAMING_QUICK_REFERENCE (10 min)
  │      ↓
  │   See key changes
  │      ↓
  ├─→ Pick your role path:
  │   │
  │   ├─ DECISION MAKER?
  │   │    → NAMING_SCHEMA Part 3-4 (30 min)
  │   │    → Done! (35 min total)
  │   │
  │   ├─ DEVELOPER?
  │   │    → NAMING_SCHEMA (45 min)
  │   │    → NAMING_MIGRATION (60 min)
  │   │    → Start coding (90+ min)
  │   │
  │   └─ USER/LEARNER?
  │        → NAMING_QUICK_REFERENCE (repeat)
  │        → NAMING_VISUAL_REFERENCE (20 min)
  │        → Done! (30 min total)
  │
  └─→ COMPLETE
```

---

**Welcome to the xs Naming Schema Documentation! Pick your entry point above and start reading.** 🚀
