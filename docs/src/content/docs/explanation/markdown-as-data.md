---
title: Markdown as data
description: Why the docs are plain Markdown, read as data and laid out by the server
sidebar:
  order: 3
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
out exactly as before. When the walk meets a group of tabs, or a code block
followed by its output, it draws those differently.

## The marks

Each mark sits where it applies.

A group of tabs opens and closes with an HTML comment, like a pair of braces.
The first heading inside sets the level, and each heading at that level becomes
a tab, labelled with its text:

```markdown
<!-- tabs -->

### brew
...
### cargo
...

<!-- /tabs -->
```

Anything after the closing comment is outside the group, so a note that
follows the tabs shows whichever tab is picked.

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

This page uses both. Here is one command in two shells, with the group
between a pair of comments, and each output block marked `output`:

## Two shells

<!-- tabs -->

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

<!-- /tabs -->

## On GitHub

Both marks are things GitHub already passes over. It hides HTML comments, and
it reads only the first word after a fence's backticks, so `json output`
highlights as JSON. On GitHub the install section reads as a heading per
method, one after another, which is a fine way to read it.

That rules out most of the syntax other tools use for the same job. Markdoc
writes `{% tabs %}`, Pandoc and Djot write `::: tabs`, MyST writes
```` ```{tab-set} ````, Nuxt Content writes `::tabs`. Each is clear to its own
renderer and shows as stray text everywhere else.

## Alternatives

The first version named the tabbed sections in the frontmatter, `tabs:
[install]`, and each tab ran to the next heading. The frontmatter could open a
group but not close it, so a note after the tabs landed in the last one. A
`---` block can't be reopened mid-page either: both Nushell's parser and GitHub
read it as a rule and a heading. Comments close the group where it ends.

Comments on their own didn't work at first: `.md` escapes HTML, comments
included, so it printed the marker on the page. Reading the page as data fixed
that, since the walk takes the comment out before `.md` sees anything.

A link definition, `[//]: # (tabs)`, is hidden on GitHub too, and reaches the
walk as a node of its own. It was passed over for comments, which read more
plainly.

A `console` code block is GitHub's convention for a terminal session: lines
starting `$ ` are commands, the rest is their output. There is no session
language for Nushell, and the output can't be highlighted as JSON.

MDX-style tags, kept and handled by Nushell closures, would have left the old
pages almost untouched. GitHub drops tags it doesn't know, so the pages still
read. They were set aside for the data approach, which needs nothing new in
`.md` and keeps the Markdown free of markup.

## Limits

A group without its closing comment runs to the end of the page. Tabs can't
sit inside other tabs: the first closing comment ends the group.

## The code

The renderer is `docs/markdown.nu`, a Nushell module of a little over 100 lines, used by
`docs/serve.nu`. It could move into http-nu's standard library, beside
`http-nu/router` and `http-nu/datastar`, once more than one site wants it.
