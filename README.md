<!-- LOGO -->
<h1>
<p align="center">
  <a href="https://cablehead.github.io/xs/">
    <img src="https://github.com/user-attachments/assets/f0c019ad-885d-4837-b72b-ef6ff1f85c0f" alt="Logo" width="400">
  </a>
  <br>cross.stream
</h1>
  <p align="center">
    Local-first event streaming for building reactive workflows and automation.
    <br />
    <a href="#about">About</a>
    ·
    <a href="https://cablehead.github.io/xs/">Documentation</a>
    ·
    <a href="#connect">Connect</a>
    ·
    <a href="#built-with-">Built with</a>
  </p>
</p>

[![CI](https://github.com/cablehead/xs/actions/workflows/ci.yml/badge.svg)](https://github.com/cablehead/xs/actions/workflows/ci.yml)
[![Discord](https://img.shields.io/discord/1182364431435436042?logo=discord)](https://discord.com/invite/YNbScHBHrh)

## About

cross.stream is a local-first event stream store that turns any CLI tool into a
reactive component. Stream data through Nushell pipelines, spawn external
processes as generators, and build handlers that automatically respond to
events.

Unlike traditional databases that store state, cross.stream captures the _flow_
of events over time. This enables powerful patterns like real-time monitoring,
automated workflows, and seamless integration between different tools and
services. Think of it as the missing piece that connects your favorite CLI tools
into a cohesive, reactive system.

Whether you're building personal automation, prototyping distributed systems, or
just want to pipe data through creative workflows, cross.stream provides the
foundation for event-driven computing on your local machine.

## Quick Start

See the
[installation guide](https://cablehead.github.io/xs/getting-started/installation/)
to get started.

## Features

- **Reactive Workflows**: Build handlers that automatically respond to events as
  they flow through the stream
- **CLI Integration**: Turn any command-line tool into a streaming component
  with generators
- **Nushell Native**: First-class integration with Nushell for powerful data
  processing pipelines
- **Real-time Streaming**: Subscribe to live event feeds and build responsive
  applications
- **Content Addressable**: Efficient storage and deduplication of large payloads
- **Local-first**: Your data stays on your machine, no cloud dependencies
  required

## Connect

Join our [Discord](https://discord.com/invite/YNbScHBHrh) to ask questions or
share ideas.

## Built with 🙏💚

- [fjall](https://github.com/fjall-rs/fjall): for indexing and metadata
- [cacache](https://github.com/zkat/cacache-rs): for content (CAS)
- [hyper](https://hyper.rs/guides/1/server/echo/): provides an HTTP/1.1 API over
  a local Unix domain socket for subscriptions, etc.
- [Nushell](https://www.nushell.sh): for scripting and
  [interop](https://utopia.rosano.ca/interoperable-visions/)
