---
title: Markdown as data
description: Why the docs are plain Markdown, read as data and laid out by the server
sidebar:
  order: 3
tabs: [two-shells]
---

A docs page is mostly prose and code, which Markdown handles well. It can't say
that four install methods are alternatives and only one matters to you, or that
a code block is the output of the command above it. Most docs tools
answer that by extending Markdown. These docs answer it by reading Markdown as
data, and leaving the layout to the server.

## Background

The xs docs used to be built with Astro and MDX. MDX lets a page drop a
component into the Markdown, `<Tabs>` with a `<TabItem>` per choice, and a
JavaScript build turns each component into HTML. It works, but it needs Node
and npm, and a build step between editing a page and seeing it.

http-nu already renders Markdown with `.md`, on each request. What it lacked was
a way to say "these sections are tabs" or "this block is output" without
inventing new syntax.

## A page, as data

Nushell parses Markdown itself. `from md --verbose` returns a page as a list of
nodes: the frontmatter, each heading with its level, each code block with its
language, each node with the lines it came from.

```nushell
open --raw first-stream.md | from md --verbose | where type == code | first | reject children
```

```text output
{
  type: code,
  position: {start: {line: 29, column: 1}, end: {line: 31, column: 4}},
  attrs: {value: "xs serve ./store", lang: bash, fence: true}
}
```

Once a page is a list, layout is ordinary code. `markdown.nu` walks the nodes.
Most go straight to `.md`, a node at a time, so highlighting and tables come
out exactly as before. When the walk meets a section named for tabs, or a code
block followed by its output, it draws those differently.

## The marks

Each mark sits where it applies.

A section has a name, its heading, so tabs are named in the frontmatter. Every
heading one level down inside that section becomes a tab, labelled with the
heading's text:

```markdown
---
tabs: [install]
---

## Install

### brew
...
### cargo
...
```

A code block has no name, so output is marked on the block itself, with a
second word after the language:

````markdown
```bash
echo -n | xs append ./store notes --meta '{"text":"a quick note"}'
```

```json output
{"topic":"notes","id":"03d4qic9vqkve1krajjtlbavd", ...}
```
````

This page uses both. Here is one command in two shells. The tabs are named
`two-shells` in this page's frontmatter, and each output block says `output`:

## Two shells

### nushell

```nushell
null | .append notes --meta {text: "a quick note"}
```

```text output
───────┬─────────────────────────────
 topic │ notes
 id    │ 03d4q1qhbiv09ovtuhokw5yxv
 meta  │ {text: "a quick note"}
───────┴─────────────────────────────
```

### bash

```bash
echo -n | xs append ./store notes --meta '{"text":"a quick note"}'
```

```json output
{"topic":"notes","id":"03d4qic9vqkve1krajjtlbavd","meta":{"text":"a quick note"}}
```

## On GitHub

Both marks are things GitHub already passes over. It shows the frontmatter as a
small table at the top of the file, and it reads only the first word after a
fence's backticks, so `json output` highlights as JSON. On
GitHub the install section reads as a heading per method, one after another,
which is a fine way to read it.

That rules out most of the syntax other tools use for the same job. Markdoc
writes `{% tabs %}`, Pandoc and Djot write `::: tabs`, MyST writes
```` ```{tab-set} ````, Nuxt Content writes `::tabs`. Each is clear to its own
renderer and shows as stray text everywhere else.

## Alternatives

An HTML comment around the tabs, `<!-- tabs -->`, is invisible on GitHub. But
`.md` escapes HTML, comments included, so it printed the marker on the page.

A `console` code block is GitHub's convention for a terminal session: lines
starting `$ ` are commands, the rest is their output. There is no session
language for Nushell, and the output can't be highlighted as JSON.

MDX-style tags, kept and handled by Nushell closures, would have left the old
pages almost untouched. GitHub drops tags it doesn't know, so the pages still
read. They were set aside for the data approach, which needs nothing new in
`.md` and keeps the Markdown free of markup.

## Limits

A tab runs to the next heading. So whatever follows a group of tabs inside the
same section lands in the last tab. A note placed after the tabs has to move
above them, or under a heading of its own.

The frontmatter can only name sections, by their headings. Two sections with
the same heading on one page would both become tabs.

## The code

The renderer is `docs/markdown.nu`, a Nushell module of a little over 100 lines, used by
`docs/serve.nu`. It could move into http-nu's standard library, beside
`http-nu/router` and `http-nu/datastar`, once more than one site wants it.
