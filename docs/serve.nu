# xs docs, served by http-nu from the Markdown in content/.
#
#   http-nu :3045 docs/serve.nu
#
# Each folder under content/ is a sidebar section; each file a page, ordered
# by its frontmatter's sidebar.order.

use http-nu/datastar *
use markdown.nu *

const DIR = path self | path dirname
const CONTENT = $DIR | path join src content docs
const BASE = "/docs"

const SECTIONS = [
  [dir label];
  [getting-started "Getting Started"]
  [tutorials Tutorials]
  [how-to "How-to Guides"]
  [explanation Explanation]
  [reference Reference]
]

# Split a page into its frontmatter and its Markdown body.
def read-page [file: path] {
  let parts = open --raw $file | parse --regex '(?s)^---\n(?<meta>.*?)\n---\n(?<body>.*)' | first
  {meta: ($parts.meta | from yaml) body: $parts.body}
}

# Every page, in sidebar order: {section, slug, title, order, file}.
def pages [] {
  $SECTIONS | each {|s|
    let dir = $CONTENT | path join $s.dir
    if not ($dir | path exists) { return [] }
    ls $dir | where name =~ '\.mdx?$' | each {|f|
      let meta = (read-page $f.name).meta
      {
        section: $s.label
        slug: $"($s.dir)/($f.name | path parse | get stem)"
        title: $meta.title
        order: ($meta.sidebar?.order? | default 999)
        file: $f.name
      }
    } | sort-by order
  } | flatten
}

def sidebar [current: string] {
  pages | group-by --to-table section | each {|g|
    let items = $g.items | each {|p|
      let here = if $p.slug == $current { ' aria-current="page"' } else { "" }
      $'<li><a href="($BASE)/($p.slug)/"($here)>($p.title | escape-html)</a></li>'
    } | str join
    $"<h2>($g.section)</h2><ul>($items)</ul>"
  } | str join
}

def escape-html []: string -> string {
  str replace -a '&' '&amp;' | str replace -a '<' '&lt;' | str replace -a '>' '&gt;'
}

# The bar across the top of every page: the name, search, links out.
def topbar [] {
  # Datastar expressions hold single quotes, which this string can't
  let empty = "''"
  let get = "@get('" + $BASE + "/search')"
  $'<header data-signals:q="($empty)">
<a class="wordmark" href="($BASE)/">cross.<span>stream</span></a>
<small>xs docs</small>
<search>
<input type="search" placeholder="Search the docs" aria-label="Search the docs" data-bind:q data-on:input__debounce.200ms="($get)">
<div id="results"></div>
</search>
<a href="https://www.cross.stream/">cross.stream</a>
<a href="https://github.com/cablehead/xs">GitHub</a>
<a href="https://discord.com/invite/YNbScHBHrh">Discord</a>
</header>'
}

def head [title: string] {
  $'<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>($title | escape-html)</title>
<link rel="stylesheet" href="($BASE)/stellar.css">
<link rel="stylesheet" href="($BASE)/syntax.css">
<link rel="stylesheet" href="($BASE)/docs.css">
<script type="module" src="($DATASTAR_JS_PATH)"></script>
</head>'
}

def layout [title: string current: string body: string] {
  $'(head $"($title) - xs docs")
<body>
(topbar)
<nav id="pages">(sidebar $current)</nav>
<main><h1>($title | escape-html)</h1>($body)</main>
</body>
</html>'
}

# The home page: the splash page's header, then a tile for each section.
const BLURBS = {
  "Getting Started": "Install xs and append your first frame."
  "Tutorials": "Build something with xs, a step at a time."
  "How-to Guides": "Recipes for a particular job."
  "Explanation": "Why xs works the way it does."
  "Reference": "Every command, option and built-in."
}

def home [] {
  let all = pages
  let tiles = $SECTIONS | each {|s|
    let first = $all | where section == $s.label | get 0?
    if $first == null { return "" }
    $'<li><a href="($BASE)/($first.slug)/"><h2>($s.label)</h2><p>($BLURBS | get $s.label)</p></a></li>'
  } | str join
  $'(head "xs docs")
<body class="home">
(topbar)
<main>
<section class="hero">
<h1><span class="wordmark">cross.<span>stream</span></span></h1>
<p><code>xs</code> is an event stream store for personal, local-first use</p>
<nav>
<a href="($BASE)/getting-started/installation/">Get started</a>
<a href="https://github.com/cablehead/xs">View on GitHub</a>
<a href="https://discord.com/invite/YNbScHBHrh">Join the Discord</a>
</nav>
</section>
<ul class="tiles">($tiles)</ul>
</main>
</body>
</html>'
}

# Search: every page whose title or text holds the words, the title matches
# first, each with the line it was found on. Read from the Markdown on each
# request, as the pages are.
def search [q: string] {
  let words = $q | str downcase | split words | where ($it | str length) > 1
  if ($words | is-empty) { return '<div id="results"></div>' }
  let hits = pages | each {|p|
    let title = $p.title | str downcase
    let doc = read-page $p.file
    let lines = $doc.body | lines | where {|l| ($l | str trim) != "" and not ($l | str starts-with "```") }
    let found = $lines | where {|l| let low = $l | str downcase; $words | all {|w| $low | str contains $w } }
    let in_title = $words | all {|w| $title | str contains $w }
    if (not $in_title) and ($found | is-empty) { return null }
    # the line it was found on, or the page's description when only the title matched
    {page: $p score: ((if $in_title { 100 } else { 0 }) + ($found | length)) line: ($found | get 0? | default ($doc.meta.description? | default ""))}
  } | compact | sort-by score --reverse | first 8
  if ($hits | is-empty) { return $'<div id="results"><p>Nothing matches ($q | escape-html).</p></div>' }
  let items = $hits | each {|h|
    # the line as text: a link keeps its words, Markdown marks go
    let snippet = $h.line | str replace -ra '\[([^\]]*)\]\([^)]*\)' '$1' | str replace -ra '^[#>|\s-]+|[`*]' '' | str trim | str substring 0..140 | escape-html
    $'<li><a href="($BASE)/($h.page.slug)/"><span>($h.page.title | escape-html)</span> <small>($h.page.section)</small><p>($snippet)</p></a></li>'
  } | str join
  $'<div id="results"><ul>($items)</ul></div>'
}

# The demo page: for each example, its Markdown, the code that lays it out,
# and the result.
def demo [] {
  let src = open --raw ($DIR | path join markdown.nu)
  let code = {|name| $src | parse --regex ('(?s)# demo:' + $name + '\n(?<c>.*?)# /demo:' + $name) | get 0.c | str trim }
  [[name title]; [tabs Tabs] [session "Command and output"]] | each {|d|
    let md = open --raw ($DIR | path join demo $"($d.name).md")
    $'<h2>($d.title)</h2>
<h3>The Markdown, demo/($d.name).md</h3>
<pre class="source">($md | escape-html)</pre>
<h3>The code, in markdown.nu</h3>
<pre class="source">(do $code $d.name | escape-html)</pre>
<h3>Rendered</h3>
<div class="rendered">(md render $md)</div>'
  } | str join "\n"
}

{|req|
  let path = $req.path | str replace -r $'^($BASE)' ''
  let slug = $path | str trim --char '/'
  let page = pages | where slug == $slug | get 0?
  if $page != null {
    let p = read-page $page.file
    layout $page.title $page.slug (md render (open --raw $page.file))
  } else if $slug == "search" {
    # a GET carries its signals in the query, so there is no body to read
    let q = "" | from datastar-signals $req | get q? | default ""
    search $q | to datastar-patch-elements | to sse
  } else if $slug == "syntax.css" {
    # code colours: Dracula in dark mode, as the Astro docs used; GitHub's
    # in light mode, against Sunlit brick
    $"@media \(prefers-color-scheme: dark\) {(.highlight theme Dracula)}\n@media \(prefers-color-scheme: light\) {(.highlight theme GitHub)}" | metadata set { merge {'http.response': {headers: {content-type: "text/css"}}} }
  } else if ($path | str starts-with "/assets/") {
    # the pages link images as ../../../assets/..., which from /docs/<section>/<page>/ lands here
    .static ($DIR | path join src) $path
  } else if $slug == "demo" {
    layout "Layout in Markdown" "demo" (demo)
  } else if $slug == "" {
    home
  } else {
    .static $DIR $path
  }
}
