---
title: Your First Stream
description: Create and interact with your first event stream
sidebar:
  order: 2
---

Let's create your first event stream.

## Start the Server

```bash
xs serve ./store
```

## Add Your First Event

```bash
echo "Hello, cross.stream!" | xs append ./store my-first-topic
```

```
─#─┬──topic───┬────────────id─────────────┬────────────────────────hash─────────────────────────┬─meta─┬───ttl───
 0 │ xs.start │ 03d4hooo81nspul7m6x0844uu │                                                     │      │
 1 │ notes    │ 03d4hoqffuo0qxk24d5wnjn7f │ sha256-wIcRiyKpOjA1Z8O+wZvoiMXYgGEzPQOhlA8AOptOhBY= │      │ forever
───┴──────────┴───────────────────────────┴─────────────────────────────────────────────────────┴──────┴─────────
```

## Watch Your Stream
```bash
xs cat ./store
```
## Add Another Event
```bash
echo "Event number two" | xs append ./store my-first-topic
```

## Follow Updates Live
```bash
xs cat --follow ./store
```

Now any new events will appear in real-time!
