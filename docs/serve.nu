# xs docs, served by http-nu from the Markdown in content/.
#
#   http-nu :3045 docs/serve.nu
#
# Each folder under content/ is a sidebar section; each file a page, ordered
# by its frontmatter's sidebar.order.

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

def layout [title: string current: string body: string] {
  $'<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>($title | escape-html) - xs docs</title>
<link rel="stylesheet" href="($BASE)/docs.css">
</head>
<body>
<nav>(sidebar $current)</nav>
<main><h1>($title | escape-html)</h1>($body)</main>
</body>
</html>'
}

{|req|
  let path = $req.path | str replace -r $'^($BASE)' ''
  let slug = $path | str trim --char '/'
  let page = pages | where slug == $slug | get 0?
  if $page != null {
    let p = read-page $page.file
    layout $page.title $page.slug ($p.body | .md | get __html)
  } else if $slug == "" {
    let first = pages | first
    "" | metadata set { merge {'http.response': {status: 302 headers: {location: $"($BASE)/($first.slug)/"}}} }
  } else {
    .static $DIR $path
  }
}
