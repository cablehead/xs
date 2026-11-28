# Runs the docs' examples against a real xs and compares what it prints with
# what the page shows. An example is a nushell code block followed by a block
# marked `output` (```text output, ```json output). Each page runs on a fresh
# store, its examples in page order, in one Nushell session, so a later block
# sees an earlier block's `let`.
#
#   nu docs/check-examples.nu <xs binary>
#
# Before comparing, both sides are cut to their words (no table borders or
# spacing) and frame ids, hashes and times are masked, since those change on
# every run. A command that follows the stream (`-f`) never ends, so it's
# skipped. Exits 1 if any example's output differs.

const DIR = path self | path dirname
const CONTENT = $DIR | path join src content docs
const XS_NU = $DIR | path join .. xs.nu

# The code blocks of a page in order, each with the output block after it,
# if one is marked. Blocks without one still run: a tutorial sets things up
# in them.
def examples [file: path]: nothing -> list {
  let nodes = open --raw $file | from md --verbose | where type == code
  let is_output = {|x| $x != null and ($x.attrs.meta? | default "" | split words | any {|w| $w == "output" }) }
  $nodes | enumerate | each {|e|
    let n = $e.item
    if (do $is_output $n) { return null }
    let next = $nodes | get -o ($e.index + 1)
    {lang: ($n.attrs.lang? | default "") code: $n.attrs.value expected: (if (do $is_output $next) { $next.attrs.value } else { null }) line: $n.position.start.line}
  } | compact
}

# Words only, with what changes on every run masked.
def words []: string -> list<string> {
  $in
  | str replace -ra '\b[0-9a-z]{25}\b' '<id>'
  | str replace -ra 'sha256-[A-Za-z0-9+/=_-]+' '<hash>'
  | str replace -ra '\d{4}-\d\d-\d\dT[\d:.]+(Z|[+-]\d\d:\d\d)?' '<time>'
  | str replace -ra '\d+ (sec|second|min|minute|hour|day)s? ago|now' '<ago>'
  | str replace -ra '[─│┬┴┼╭╮╰╯├┤═║╔╗╚╝]' ' '
  # a record shows as a nested table or inline, {text: "..."}, depending on
  # width: the words are the same either way
  | str replace -ra '[{}\[\]",:]' ' '
  | split row -r '\s+' | where $it != ""
}

# One block as a statement of the page's script: run, its result printed
# after a marker, so each block's output can be told apart, and a pause for
# services and actors, which answer a moment later. A block's error is
# printed in its place and the page goes on.
def as-step [i: int, code: string, shown: bool]: nothing -> string {
  let lines = $code | str trim | lines
  let marker = $"print '<<<example ($i)>>>'"
  let show = if $shown { "table --expand --width 400 | print" } else { "ignore" }
  # a block that declares variables keeps them for later blocks: its last
  # line is the value shown
  let run = if ($lines | any {|l| $l | str starts-with "let " }) {
    $"($lines | drop 1 | str join "\n")\n(($lines | last)) | ($show)"
  } else {
    $"try { do {\n($code)\n} | ($show) } catch {|e| print $'<<<error: \(\$e.msg\)>>>' }"
  }
  $"($marker)\n($run)\nsleep 500ms"
}

def run-page [xs: path, file: path]: nothing -> list {
  let exs = examples $file | where lang == "nushell"
  if ($exs | where expected != null | is-empty) { return [] }
  # a short path: a Unix socket's path can't be longer than about 104 characters on macOS
  let store = mktemp -d --tmpdir-path /tmp xsdoc.XXXX
  let job = job spawn { ^$xs serve $store out+err> ($store | path join serve.log) }
  # the store is up once its API socket exists
  for _ in 1..50 { if ($store | path join sock | path exists) { break }; sleep 100ms }
  let steps = $exs | enumerate | each {|e|
    # a piece of code shown on its own (one arm of a match) can't run alone,
    # and a block that doesn't parse would stop the whole page
    # (a variable from an earlier block isn't known to the check, and that's fine)
    let parses = try { $e.item.code | nu-check --debug; true } catch {|err| ($err.debug? | default "") =~ "(?i)variable not found" }
    if not $parses { $"print '<<<example ($e.index)>>>'\nprint '<<<skipped: a fragment, not a whole command>>>'" } else if ($e.item.code =~ '(^|\s)-f(\s|$)|--follow') { $"print '<<<example ($e.index)>>>'\nprint '<<<skipped: follows the stream>>>'" } else { as-step $e.index $e.item.code ($e.item.expected != null) }
  }
  let script = $"use ($XS_NU) *\n$env.XS_ADDR = '($store)'\n($steps | str join "\n")"
  # xs.nu runs `xs`, so the one being checked goes first on the PATH
  # The page's commands run from the docs' content folder, so an example
  # can open a page by its path there.
  # a block that never ends (a server, a stream followed without -f) would
  # hang the check: a page gets two minutes, and what it printed so far
  # counts. Its input is empty, as at a prompt: `.append` with nothing piped
  # in would otherwise wait on this script's input for ever.
  let out = with-env {PATH: ($env.PATH | prepend ($xs | path dirname))} { cd $CONTENT; do { "" | ^perl -e 'alarm shift; exec @ARGV' 120 nu --no-config-file -c $script } | complete }
  job kill $job
  rm -rf $store
  let parts = $out.stdout | split row -r '<<<example \d+>>>\n?' | skip 1
  $exs | enumerate | where item.expected != null | each {|e|
    let got = $parts | get -o $e.index | default "<<<error: never ran: the page stopped before it, or ran out of time>>>"
    let where = $"($file | path relative-to $CONTENT):($e.item.line)"
    if ($got | str contains "<<<skipped") { {at: $where result: skipped note: ($got | parse --regex '<<<skipped: (?<w>[^>]*)>>>' | get 0?.w? | default "")} } else if ($got | str contains "<<<error:") {
      {at: $where result: error note: ($got | str trim | str substring 0..300)}
    } else if ($got | words) == ($e.item.expected | words) {
      {at: $where result: pass note: ""}
    } else {
      let g = $got | words; let x = $e.item.expected | words
      {at: $where result: differs note: $"only in xs's output: ($g | where $it not-in $x | uniq | str join ' ')" expected: $"only on the page: ($x | where $it not-in $g | uniq | str join ' ')"}
    }
  } | append (if $out.exit_code != 0 and ($out.stderr | str trim | is-not-empty) { [{at: ($file | path relative-to $CONTENT) result: error note: ($out.stderr | str trim | str substring 0..400)}] } else { [] })
}

def main [xs: path] {
  let files = glob ($CONTENT | path join "**/*.md") | sort
  let results = $files | each {|f| run-page ($xs | path expand) $f } | flatten
  $results | each {|r|
    print $"($r.result | fill -w 8) ($r.at)"
    if $r.result in [differs error] {
      print $"   ($r.note | str replace -a "\n" ' ⏎ ')"
      if ($r.expected? != null) { print $"   ($r.expected | str replace -a "\n" ' ⏎ ')" }
    }
  }
  let n = $results | group-by result | transpose k v | each {|g| $"($g.v | length) ($g.k)" } | str join ", "
  print $n
  if ($results | any {|r| $r.result in [differs error] }) { exit 1 }
}
