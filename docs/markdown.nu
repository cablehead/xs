# --- render: Markdown as data, laid out by this code ---------------------
#
# A page is read with `from md --verbose`: a list of nodes, each with its
# type, its lines in the file, and (for code) its language and extra words.
# Most nodes render as they are. Two kinds of group draw differently:
#   - tabs: a section named in the frontmatter's `tabs` list, by its
#     heading; each heading one level down inside it becomes a tab, labelled
#     with that heading. Groups with the same labels switch together.
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
def tabs-html [lines: list<string> level: int nodes: list] {
  # split the section's nodes at each heading one level down
  let tab_head = $"h($level + 1)"
  # what comes before the first tab heading draws as usual, above the tabs
  let lead = $nodes | take while {|n| $n.type != $tab_head }
  let tabs = $nodes | skip ($lead | length) | reduce --fold [] {|n acc|
    if $n.type == $tab_head {
      $acc | append {label: ($n | node-text) nodes: []}
    } else if ($acc | is-empty) {
      $acc
    } else {
      $acc | update (($acc | length) - 1) {|t| $t | update nodes ($t.nodes | append $n) }
    }
  }
  # groups with the same labels share one signal, so they switch together
  let signal = "tab_" + ($tabs | get label | sort | str join "_" | str replace -ra '[^a-zA-Z0-9_]' '_')
  let first = $tabs | first | get label
  let buttons = $tabs | each {|t|
    $"<button role=\"tab\" data-on:click=\"$($signal) = '($t.label)'\" data-attr:aria-selected=\"$($signal) == '($t.label)'\">($t.label)</button>"
  } | str join
  let panels = $tabs | each {|t|
    $"<section role=\"tabpanel\" data-show=\"$($signal) == '($t.label)'\">(render-nodes $lines $t.nodes [])</section>"
  } | str join
  (render-nodes $lines $lead []) + $"<div class=\"tabs\" data-signals:($signal)=\"'($first)'\"><div role=\"tablist\">($buttons)</div>($panels)</div>"
}
# /demo:tabs

# demo:session
def session-html [lines: list<string> command: record output: record] {
  $'<figure class="session">(md-of $lines [$command])(md-of $lines [$output])</figure>'
}
# /demo:session

# Draw a run of nodes: command and output pairs as one figure, tab sections
# as tabs, everything else as it is.
def render-nodes [lines: list<string> nodes: list tabbed: list]: nothing -> string {
  mut out = []
  mut i = 0
  let count = $nodes | length
  while $i < $count {
    let n = $nodes | get $i
    let next = $nodes | get -o ($i + 1)
    if $n.type == "code" and $next != null and $next.type == "code" and ($next.attrs.meta? | default "" | split words | any {|w| $w == "output" }) {
      $out = ($out | append (session-html $lines $n $next))
      $i = $i + 2
    } else if ($n.type =~ '^h[1-6]$') and (($n | node-text | slugify) in $tabbed) {
      # the section runs to the next heading at its level or above
      let level = $n.attrs.level
      let rest = $nodes | skip ($i + 1)
      let len = $rest | take while {|x| not ($x.type =~ '^h[1-6]$' and $x.attrs.level <= $level) } | length
      $out = ($out | append (md-of $lines [$n]) | append (tabs-html $lines $level ($rest | first $len)))
      $i = $i + 1 + $len
    } else {
      $out = ($out | append (md-of $lines [$n]))
      $i = $i + 1
    }
  }
  $out | str join "\n"
}

# `from md --verbose` lists a paragraph's text, links and inline code, and
# a table's or list's lines, as separate nodes. Merge nodes that share a line,
# or sit on the next line with no blank between, into one block, and the
# items of one list across blank lines. Headings and
# code blocks stay apart, since tabs and command blocks are found by them.
def blocks [lines: list<string> nodes: list]: nothing -> list {
  $nodes | reduce --fold [] {|n acc|
    let last = $acc | last
    let apart = ($n.type =~ '^(h[1-6]|code)$') or ($last != null and ($last.type =~ '^(h[1-6]|code)$'))
    # a list whose items have blank lines between them is still one list
    let same_list = $last != null and $n.type == "list" and $last.type == "list" and ($lines | slice ($last.position.end.line)..($n.position.start.line - 2) | all {|l| ($l | str trim) == "" })
    if $last != null and ($n.position.start.line <= $last.position.end.line or $same_list or (not $apart and $n.position.start.line == $last.position.end.line + 1)) {
      let end = [$last.position.end.line $n.position.end.line] | math max
      $acc | update (($acc | length) - 1) {|b| $b | update position.end.line $end }
    } else {
      $acc | append $n
    }
  }
}

export def "md render" [text: string]: nothing -> string {
  let fm = $text | parse --regex '(?s)^---\n(?<meta>.*?)\n---\n' | get 0?.meta? | default "" | from yaml | default {}
  let lines = $text | lines
  render-nodes $lines (blocks $lines ($text | from md --verbose | where type != yaml)) ($fm.tabs? | default [])
}
