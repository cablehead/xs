# xs

## Overview / Sketch

An event stream store for personal, local-first use. Kinda like the
[`sqlite3` cli](https://sqlite.org/cli.html), but specializing in the
[event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) use case.

![screenshot](./screenshot.png)

> "You don't so much run it, as poke _at_ it."

Built with:

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)

## Built-in Topics

- `xs.start`: emitted when the server mounts the stream to expose an API
- `xs.stop`: emitted when the server stops :: TODO

- `xs.pulse`: (synthetic) a heartbeat event you can configure to be emitted every
  N seconds when in follow mode

- `xs.threshold`: (synthetic) marks the boundary between
  replaying events and events that are newly arriving in real-time via a live
  subscription

- `<topic>.spawn` :: spawn a generator
    - meta:: topic: string, duplex: bool
    - `<topic>.terminate`

- `<topic>.register` :: register an event handler
    - meta:: run-from: start, tail, id?
    - `<topic>.unregister`

## Local socket HTTP API

WIP, thoughts:

- `/:topic` should probably be `/stream/:topic`

## API Endpoints

### GET

- `/` - Pull the event stream
- `/:id` - Pull a specific event by ID (where ID is a valid Scru128Id)
- `/cas/:hash` - Pull the content addressed by `hash` (where hash is a valid ssri::Integrity)

### POST

- `/:topic` - Append a new event to the stream for `topic`. The body of the POST
  will be stored in the CAS. You can also pass arbitrary JSON meta data using
  the `xs-meta` HTTP header.
- `/pipe/:id` - Execute a script on a specific event. The ID should be a valid Scru128Id,
  and the body should contain the script to be executed.

## Features

- event stream:
  - [x] append
  - [x] cat: last-id, follow, tail, threshold / heartbeat synthetic events
  - [x] get
  - [ ] last
  - [ ] first
  - [ ] next?
  - [ ] previous?
- [x] cas, get
- [x] ephemeral events / content
- [ ] content can be chunked, to accomodate slow streams, e.g server sent events
- [ ] secondary indexes for topics: the head of a topic can be used as a materialized view
- process management: you can register snippets of Nushell on the event stream.
  server facilitates watching for updates + managing processes
    - [x] generators
    - [x] handlers
- [x] builtin http server:
  - [x] You can optionally serve HTTP requests from your store. Requests are
        written to the event stream as `http.request` and then the connection
        watches the event stream for a `http.response`.
  - [x] You can register event handlers that subscribe to `http.request` events
        and emit `http.response` events.
- Ability for a single xs process to serve many stores
    - so you generally run just one locally, using the systems local process
      manager, and then add and remove stores to serve via the event stream

## Path Traveled

- [xs-3](https://github.com/cablehead/xs-3):
  [sled](https://github.com/spacejam/sled) index with
  [cacache](https://github.com/zkat/cacache-rs) CAS, no concurrency
- [xs-0](https://github.com/cablehead/xs-0) original experiment.
  -[LMDB](http://www.lmdb.tech/doc/) combined index / content store (pre
  realizing the event primary content should be stored in a CAS)
  - Multi-process concurrent, but polling for subscribe
