---
title: Exposing Streams
sidebar:
  order: 2
---

Reference for exposing xs stores for remote access and connecting to remote stores.

## Server: Exposing a Store

When running `xs serve`, you can expose your store for remote access using the `--expose` option with different networking protocols:

### TCP (Traditional Networking)

Expose on a specific host and port:

```sh
xs serve ./store --expose :3021  # localhost on port 3021
xs serve ./store --expose 192.168.1.100:8080  # specific host
```

### Iroh (Peer-to-Peer)

Expose via [iroh](https://www.iroh.computer) for peer-to-peer QUIC connections:

```sh
xs serve ./store --expose iroh://
```

When using iroh, the server will generate a connection ticket that clients can use to connect directly via QUIC, even across NATs and firewalls.

## Getting the Connection Details

### For TCP
The address you specify is the connection address:
- TCP: `:3021` or `192.168.1.100:8080`

### For Iroh
To get the iroh connection ticket, use the `.last` command:

```sh
.last xs.start
```

The ticket will be in the `expose` field of the metadata:

```
────────────┬─────────────────────────────────────────────────
 topic      │ xs.start
 id         │ 03ek4n3sq2i9fio5tc9wn9mov
 hash       │
 meta       │ ─────────┬─────────────────────────────────────
            │  expose  │ iroh://nodeaagxeomb7ezyg4prcpn...
            │ ─────────┴─────────────────────────────────────
 ttl        │
────────────┴─────────────────────────────────────────────────
```

## Client: Connecting to Remote Stores

All xs commands that accept an `<addr>` parameter can connect to remote stores:

### TCP Connections

```sh
xs cat :3021
"hello world" | xs append 192.168.1.100:8080 chat
xs eval :3021 -c '.cat --topic notes'
```


### Iroh Connections

Use the full iroh ticket from the server:

```sh
xs cat iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d
"hello p2p QUIC" | xs append iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d chat
xs eval iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d -c '.last status'
```

## Environment Configuration

### XS_ADDR Environment Variable

Set `$env.XS_ADDR` to avoid repeating the connection address in xs.nu commands:

```sh
# TCP connection
$env.XS_ADDR = ":3021"

# Iroh connection  
$env.XS_ADDR = "iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d"

```

With `XS_ADDR` set, xs.nu convenience functions will automatically use the remote store:

```sh
.cat                    # connects to $env.XS_ADDR
"message" | .append logs  # appends to remote store
.last status            # gets last from remote store
```

See the [xs.nu reference](../xs-nu/) for all available convenience functions.

## Connection Examples

### Local Development
```sh
# Terminal 1: Start server
xs serve ./store --expose :3021

# Terminal 2: Connect and use
$env.XS_ADDR = ":3021"
"development note" | .append notes
.cat --follow
```

### Peer-to-Peer QUIC Sharing
```sh
# Server: Start with iroh
xs serve ./store --expose iroh://

# Server: Get ticket
.last xs.start

# Client: Connect with ticket
xs cat iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d --follow
```

### Remote Evaluation
```sh
# Evaluate Nushell scripts on remote store
xs eval :3021 -c '{topic: "heartbeat", timestamp: (date now)}'
xs eval iroh://nodeac4lrf7r5hyqeuj...crjpx6ajinb2hi4d -c 'ls | each { $in } | .append files'
```