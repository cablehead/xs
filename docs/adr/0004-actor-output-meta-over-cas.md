# ADR 0004: Prefer Meta Over CAS for Structured Data

## Context

Frames have two places to carry data: metadata (inline on the frame) and CAS (content-addressed storage, referenced by hash). CAS requires an indirect lookup to read the value back. For a list of 50 frames, that's 50 CAS calls. Metadata is inline, no extra lookup.

CAS exists for content that benefits from it: images, large text, binary blobs, anything where deduplication or streaming reads matter. Structured data (records, small strings, numbers) gets none of those benefits. The indirection just makes it harder to work with.

## Decision

Prefer metadata for structured data. Use CAS for blobs.

Rule of thumb: if the value is something you'd put in a JSON field, it belongs in meta. If it's something you'd serve as a file, it belongs in CAS.

## Consequences

- Querying structured data is direct: `.last foo.out | get meta.total` instead of `.last foo.out | .cas $in.hash | from json | get total`
- Batch reads (`.cat --topic foo.*`) return usable data without N extra CAS lookups
- CAS stays focused on what it's good at: large and binary content
