---
title: Your First Stream
---

Append a note:

```nushell
null | .append notes --meta {text: "a quick note"}
```

```text output
───────┬─────────────────────────────
 topic │ notes
 id    │ 03d4q1qhbiv09ovtuhokw5yxv
 meta  │ {text: "a quick note"}
 ttl   │ forever
───────┴─────────────────────────────
```

The same from bash, where it comes back as JSON:

```bash
echo -n | xs append ./store notes --meta '{"text":"a quick note"}'
```

```json output
{"topic":"notes","id":"03d4qic9vqkve1krajjtlbavd","meta":{"text":"a quick note"},"ttl":"forever"}
```
