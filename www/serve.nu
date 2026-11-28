# www.cross.stream, redone: one page drawn on the server from the data
# below, styled on raw tags with tokens (site.css), and Datastar for the one
# thing that changes in the page, the query you pick.
#
#   http-nu --datastar :3021 www/serve.nu
#
# One site with the docs: /docs/ is theirs, drawn by docs/serve.nu.

use http-nu/datastar *

# (not DIR: the docs' serve.nu, sourced below, has its own)
const SITE = path self | path dirname

# the docs' own handler, for everything under /docs/
let docs = source ../docs/serve.nu

const RELEASE = {version: "v0.16.0" url: "https://github.com/cablehead/xs/releases/tag/v0.16.0" date: "2026-11-18"}

# What the demo appends, in order: each line typed, and the frame it makes.
const FRAMES = [
  [topic text id];
  [goal "buy groceries" "03egou0geu62jz78r2smufms2"]
  [notes "quick note about meeting" "03ehpx4n8q5bvz92k3wtnlem7"]
  [notes "submit TPS report" "03ejkr9z2m8xwv45n7qstpkm9"]
  [goal "learn cross.stream" "03elmb5w7p3qxt68s9vuwron2"]
  [notes "review project docs" "03enmc8v4r6zyu79t1xvwspn4"]
]

# The parts of xs, each a kind with its brand colour (n2's kind title).
const PARTS = [
  [name kind href text];
  ["Event Store" blue "/docs/explanation/architecture/" "Append-only stream with structured metadata inline on each frame. Content-addressable storage (CAS) for binary and large blobs."]
  [Actors amber "/docs/reference/actors/" "React to incoming frames with stateful processors: transform, trigger, fan-out."]
  [Actions violet "/docs/reference/actions/" "On-demand, stateless ops. Perfect for RPC-style calls or streaming data back to the caller."]
  [Services green "/docs/reference/services/" "Continuously pull data from files, APIs, websockets, serial devices, or sensors."]
  [Local-First red "/docs/getting-started/first-stream/" "Single binary, file-based store, Unix socket API. No cloud, works offline."]
  ["Nushell Integration" teal "/docs/reference/xs-nu/" "Write actors and services in Nushell with strong typing and structured pipelines."]
]

{|req|
  if $req.path == "/docs" or ($req.path | str starts-with "/docs/") {
    do $docs $req
  } else if ($req.path | str starts-with "/assets/") and not ($SITE | path join ($req.path | str trim --left --char "/") | path exists) {
    # the docs' images: their Markdown links them as ../../../assets/..., a
    # path that from /docs/<section>/<page>/ lands here
    do $docs $req
  } else if $req.path == "/" {
    let queries = [
      {id: notes title: "Get all notes" comment: "Filter events to show only notes entries" cmd: '.cat | where topic == "notes" | each { $in.meta.text }' rows: ($FRAMES | where topic == notes | get text)}
      {id: goal title: "Get current goal" comment: "The latest goal: the last frame on its topic" cmd: '.last goal | $in.meta.text' rows: [($FRAMES | where topic == goal | last | get text)]}
    ]
    # the stylesheets' version, so a browser fetches them again when either changes
    let css_v = [stellar.css site.css] | each {|f| open --raw ($SITE | path join $f) } | str join | hash md5 | str substring 0..7
    {frames: $FRAMES parts: $PARTS release: $RELEASE queries: $queries datastar: $DATASTAR_JS_PATH css_v: $css_v}
    | .mj ($SITE | path join index.html)
  } else {
    .static $SITE $req.path
  }
}
