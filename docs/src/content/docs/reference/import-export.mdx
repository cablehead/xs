---
title: Import & Export
description: "How to export and import data between cross.stream stores"
sidebar:
  order: 6
---

The supervisor exposes two endpoints to facilitate data transfer between stores:

- POST `/import`: Takes JSON frame data and imports it as-is, preserving frame `id` and content `hash`
- POST `/cas`: Stores posted content in CAS and returns its hash

## Commands

`xs.nu` provides two commands to utilize these endpoints:

```nushell
# Export store at $env.XS_ADDR to path
.export <path>

# Import dump at path to $env.XS_ADDR
.import <path>
```

The exported data includes:
- Frame metadata in `frames.jsonl`
- Content files in `cas/` directory

These `xs.nu` commands operate on a whole export directory.

## From eval and handlers

Inside `xs eval` and handler scripts (actions, actors, services) two builtins
expose the import primitives directly:

- `.import` inserts a single frame verbatim, preserving its `id`. It is the
  in-engine equivalent of the `xs import` CLI and the `POST /import` endpoint,
  and the primitive the directory-level `.import <path>` helper is built on.
- `.cas-post` writes the pipeline input to the CAS and returns its hash, the
  write counterpart to `.cas` and the in-engine equivalent of `POST /cas`.

```nushell
# pipe in a frame record (or its JSON) to restore it
$frame | .import

# stash content in the CAS, keep the hash to reference from a frame
let hash = ("some content" | .cas-post)
```

## Version Compatibility

Version 0.1.0 was the first version supporting imports, though the 0.1.0 client can export data from 0.0.9 stores.

## Example

```sh
# Export from remote store
with-env {XS_ADDR: "https://user:token@remote-store.example.com"} {
    .export backup
}

# Import to local store
with-env {XS_ADDR: "./store"} {
    .import backup
}
```
