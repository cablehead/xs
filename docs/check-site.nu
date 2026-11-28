# Checks the docs site as a reader meets it: every page loads, none shows
# a component as raw text, and every link between pages and every image
# leads somewhere.
#
#   nu docs/check-site.nu http://127.0.0.1:3045
#
# Exits 1 and lists what's broken, if anything is.

const DIR = path self | path dirname
const CONTENT = $DIR | path join src content docs

# A path relative to a page, made absolute: ../a/ from /docs/b/c/ is /docs/b/a/.
def resolve [base: string, rel: string]: nothing -> string {
  if ($rel | str starts-with "/") { return $rel }
  let parts = ($base | split row "/" | drop 1) ++ ($rel | split row "/")
  $parts | reduce --fold [] {|p, acc|
    if $p == ".." { $acc | drop 1 } else if $p == "." { $acc } else { $acc | append $p }
  } | str join "/"
}

def status [url: string]: nothing -> int {
  try { http get --full --allow-errors $url | get status } catch { 0 }
}

def main [site: string] {
  let pages = glob ($CONTENT | path join "**/*.md")
    | each {|f| $f | path relative-to $CONTENT | str replace -r '\.md$' '' }
    | where $it != "index"
  mut broken = []
  mut targets = {}
  for slug in $pages {
    let path = $"/docs/($slug)/"
    let r = try { http get --full --allow-errors $"($site)($path)" } catch { {status: 0 body: ""} }
    if $r.status != 200 { $broken = ($broken | append $"($path): status ($r.status)"); continue }
    let body = $r.body
    # a component the renderer didn't know comes out escaped, as text
    let raw = $body | parse --regex '<p>&lt;(?<tag>[A-Za-z][A-Za-z0-9-]*)' | get tag | uniq
    if ($raw | is-not-empty) { $broken = ($broken | append $"($path): shows raw tags ($raw | str join ', ')") }
    # links inside the page, and its images
    let main = $body | split row "<main>" | get 1? | default ""
    let links = $main | parse --regex 'href="(?<u>[^"#]+)' | get u | where {|u| not ($u | str starts-with "http") and not ($u | str starts-with "mailto:") }
    let imgs = $main | parse --regex '<img[^>]*src="(?<u>[^"]+)"' | get u | where {|u| not ($u | str starts-with "http") }
    for u in ($links ++ $imgs) {
      let abs = resolve $path $u
      $targets = ($targets | upsert $abs ($targets | get -o $abs | default [] | append $path))
    }
  }
  for t in ($targets | transpose url from) {
    let s = status $"($site)($t.url)"
    if $s != 200 { $broken = ($broken | append $"($t.url): status ($s), linked from ($t.from | uniq | str join ', ')") }
  }
  print $"($pages | length) pages, ($targets | columns | length) links and images"
  if ($broken | is-empty) { print "all fine" } else {
    $broken | each {|b| print $"broken: ($b)" }
    exit 1
  }
}
