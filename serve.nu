# www.cross.stream, redone: one page drawn on the server from the data
# below, styled on raw tags with tokens (site.css), and Datastar for the one
# thing that changes in the page, the query you pick.
#
#   http-nu --datastar :3021 www/serve.nu

use http-nu/datastar *

const DIR = path self | path dirname

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
  ["Event Store" blue "https://cablehead.github.io/xs/explanation/architecture/" "Append-only stream with structured metadata inline on each frame. Content-addressable storage (CAS) for binary and large blobs."]
  [Actors amber "https://cablehead.github.io/xs/reference/actors/" "React to incoming frames with stateful processors: transform, trigger, fan-out."]
  [Actions violet "https://cablehead.github.io/xs/reference/actions/" "On-demand, stateless ops. Perfect for RPC-style calls or streaming data back to the caller."]
  [Services green "https://cablehead.github.io/xs/reference/services/" "Continuously pull data from files, APIs, websockets, serial devices, or sensors."]
  [Local-First red "https://cablehead.github.io/xs/getting-started/first-stream/" "Single binary, file-based store, Unix socket API. No cloud, works offline."]
  ["Nushell Integration" teal "https://cablehead.github.io/xs/reference/xs-nu/" "Write actors and services in Nushell with strong typing and structured pipelines."]
]

{|req|
  if $req.path == "/" {
    let queries = [
      {id: notes title: "Get all notes" comment: "Filter events to show only notes entries" cmd: '.cat | where topic == "notes" | each { $in.meta.text }' rows: ($FRAMES | where topic == notes | get text)}
      {id: goal title: "Get current goal" comment: "The latest goal: the last frame on its topic" cmd: '.last goal | $in.meta.text' rows: [($FRAMES | where topic == goal | last | get text)]}
    ]
    {frames: $FRAMES parts: $PARTS release: $RELEASE queries: $queries datastar: $DATASTAR_JS_PATH}
    | .mj ($DIR | path join index.html)
  } else {
    .static $DIR $req.path
  }
}
