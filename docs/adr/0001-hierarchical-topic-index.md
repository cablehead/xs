# ADR 0001: Topic Names and Hierarchical Queries

## Topic Names

Allowed characters: `a-z A-Z 0-9 _ - .`

Must start with: `a-z A-Z 0-9 _`

Cannot be empty. Cannot end with `.`. Cannot contain `..`. Max length 255 bytes.

The `.` character is the hierarchy separator.

## Wildcard Queries

Syntax: `--topic user.*` matches all topics starting with `user.`

Does not include exact topic `user` — only children. Follows NATS/Redis glob semantics.

Results ordered chronologically by frame_id.

## Index Structure

Same keyspace (`idx_topic`) stores both exact and prefix entries:
- Exact: `topic\0frame_id`
- Prefix: `prefix.\0frame_id` for each `.` segment

Topic `user.id1.messages` creates entries:
- `user.id1.messages\0frame_id` (exact)
- `user.\0frame_id` (prefix)
- `user.id1.\0frame_id` (prefix)