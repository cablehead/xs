# xs docs, served by http-nu from the Markdown in content/.
#
#   http-nu :3045 docs/serve.nu
#
# Each folder under content/ is a sidebar section; each file a page, ordered
# by its frontmatter's sidebar.order.

use http-nu/datastar *

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

# --- render: Markdown as data, laid out by this code ---------------------
#
# A page is read with `from md --verbose`: a list of nodes, each with its
# type, its lines in the file, and (for code) its language and extra words.
# Most nodes render as they are. Two kinds of group draw differently:
#   - tabs: a `##` section named in the frontmatter's `tabs` list; each
#     `###` inside it becomes a tab, labelled with the heading.
#   - command and output: a code block followed by one marked `output`
#     (```json output) is drawn as one figure.

def slugify []: string -> string {
  str downcase | str replace -ra '[^a-z0-9 -]' '' | str trim | str replace -a ' ' '-'
}

def node-text []: record -> string {
  let n = $in
  ($n.attrs?.value? | default "") + ($n.children? | default [] | each { node-text } | str join)
}

# The Markdown of nodes, from the first one's first line to the last one's last.
def md-of [lines: list<string> nodes: list] {
  if ($nodes | is-empty) { return "" }
  $lines | slice (($nodes | first).position.start.line - 1)..(($nodes | last).position.end.line - 1)
  | str join "\n" | .md | get __html
}

# demo:tabs
def tabs-html [lines: list<string> name: string nodes: list] {
  # split the section's nodes at each ### heading
  let tabs = $nodes | reduce --fold [] {|n acc|
    if $n.type == "h3" {
      $acc | append {label: ($n | node-text) nodes: []}
    } else if ($acc | is-empty) {
      $acc
    } else {
      $acc | update (($acc | length) - 1) {|t| $t | update nodes ($t.nodes | append $n) }
    }
  }
  let signal = $"tab_($name | str replace -a '-' '_')"
  let first = $tabs | first | get label
  let buttons = $tabs | each {|t|
    $"<button role=\"tab\" data-on:click=\"$($signal) = '($t.label)'\" data-attr:aria-selected=\"$($signal) == '($t.label)'\">($t.label)</button>"
  } | str join
  let panels = $tabs | each {|t|
    $"<section role=\"tabpanel\" data-show=\"$($signal) == '($t.label)'\">(md-of $lines $t.nodes)</section>"
  } | str join
  $"<div class=\"tabs\" data-signals:($signal)=\"'($first)'\"><div role=\"tablist\">($buttons)</div>($panels)</div>"
}
# /demo:tabs

# demo:session
def session-html [lines: list<string> command: record output: record] {
  $'<figure class="session">(md-of $lines [$command])(md-of $lines [$output])</figure>'
}
# /demo:session

export def render [text: string]: nothing -> string {
  let lines = $text | lines
  let nodes = $text | from md --verbose | where type != yaml
  let fm = $text | parse --regex '(?s)^---\n(?<meta>.*?)\n---\n' | get 0?.meta? | default "" | from yaml | default {}
  let tabbed = $fm.tabs? | default []
  mut out = []
  mut i = 0
  let count = $nodes | length
  while $i < $count {
    let n = $nodes | get $i
    let next = $nodes | get -o ($i + 1)
    if $n.type == "code" and $next != null and $next.type == "code" and ($next.attrs.meta? | default "" | split words | any {|w| $w == "output" }) {
      $out = ($out | append (session-html $lines $n $next))
      $i = $i + 2
    } else if $n.type == "h2" and (($n | node-text | slugify) in $tabbed) {
      # the section runs to the next ## or #
      let rest = $nodes | skip ($i + 1)
      let len = $rest | take while {|x| $x.type not-in [h1 h2] } | length
      $out = ($out | append (md-of $lines [$n]) | append (tabs-html $lines ($n | node-text | slugify) ($rest | first $len)))
      $i = $i + 1 + $len
    } else {
      $out = ($out | append (md-of $lines [$n]))
      $i = $i + 1
    }
  }
  $out | str join "\n"
}

def layout [title: string current: string body: string] {
  $'<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>($title | escape-html) - xs docs</title>
<link rel="stylesheet" href="($BASE)/docs.css">
<script type="module" src="($DATASTAR_JS_PATH)"></script>
</head>
<body>
<nav>(sidebar $current)</nav>
<main><h1>($title | escape-html)</h1>($body)</main>
</body>
</html>'
}

# The demo page: for each example, its Markdown, the code that lays it out,
# and the result.
def demo [] {
  let src = open --raw ($DIR | path join serve.nu)
  let code = {|name| $src | parse --regex ('(?s)# demo:' + $name + '\n(?<c>.*?)# /demo:' + $name) | get 0.c | str trim }
  [[name title]; [tabs Tabs] [session "Command and output"]] | each {|d|
    let md = open --raw ($DIR | path join demo $"($d.name).md")
    $'<h2>($d.title)</h2>
<h3>The Markdown, demo/($d.name).md</h3>
<pre class="source">($md | escape-html)</pre>
<h3>The code, in serve.nu</h3>
<pre class="source">(do $code $d.name | escape-html)</pre>
<h3>Rendered</h3>
<div class="rendered">(render $md)</div>'
  } | str join "\n"
}

{|req|
  let path = $req.path | str replace -r $'^($BASE)' ''
  let slug = $path | str trim --char '/'
  let page = pages | where slug == $slug | get 0?
  if $page != null {
    let p = read-page $page.file
    layout $page.title $page.slug (render (open --raw $page.file))
  } else if $slug == "demo" {
    layout "Layout in Markdown" "demo" (demo)
  } else if $slug == "" {
    let first = pages | first
    "" | metadata set { merge {'http.response': {status: 302 headers: {location: $"($BASE)/($first.slug)/"}}} }
  } else {
    .static $DIR $path
  }
}
