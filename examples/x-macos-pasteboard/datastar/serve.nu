use /Users/andy/s/03erbwsly19fej1b1zw5wv7r3/xs.nu *
use lib *

{|req|
  let body = $in
  match $req {
    {method: "GET" , path: "/syntax-highlight.css"} => {
      .response {
        headers: {
          "Content-Type": "text/css"
          "Cache-Control": "no-cache"
        }
      }
      return (m2h theme "Solarized (light)")
    }

    {method: "GET" , path: "/"} => {
      "<h1>Pasteboard Viewer</h1><ul><li><a href=\"/internal\">Internal (.mj)</a></li><li><a href=\"/external\">External (minijinja-cli)</a></li></ul>"
    }

    {method: "GET" , path: "/internal"} => {
      {updatesEndpoint: "/updates/internal", mode: "internal"} | .mj "templates/index.html.j2"
    }

    {method: "GET" , path: "/external"} => {
      {updatesEndpoint: "/updates/external", mode: "external"} | .mj "templates/index.html.j2"
    }

    {method: "GET" , path: "/updates/internal"} => {
      .response {
        headers: {
          "Content-Type": "text/event-stream"
          "Cache-Control": "no-cache"
        }
      }

      # Load items from event store
      let items = (group-pasteboard-items)
      let initial_state = {selectedId: ($items.0.id)}

      # Use generate to maintain selection state and render (internal .mj)
      .cat -T selection -f
      | process-selection $items $initial_state
      | render-sse-internal $items "templates/two-pane.html.j2"
    }

    {method: "GET" , path: "/updates/external"} => {
      .response {
        headers: {
          "Content-Type": "text/event-stream"
          "Cache-Control": "no-cache"
        }
      }

      # Load items from event store
      let items = (group-pasteboard-items)
      let initial_state = {selectedId: ($items.0.id)}

      # Use generate to maintain selection state and render (external minijinja-cli)
      .cat -T selection -f
      | process-selection $items $initial_state
      | render-sse-external $items "templates/two-pane.html.j2"
    }

    {method: "POST", path: "/select/down"} => {
      .response {status: 204}
      .append selection --meta {action: "down"} --ttl ephemeral | ignore
    }

    {method: "POST", path: "/select/up"} => {
      .response {status: 204}
      .append selection --meta {action: "up"} --ttl ephemeral | ignore
    }

    {method: "POST"} => {
      # Handle /select/<id>
      let select_match = ($req.path | parse "/select/{id}")
      if ($select_match | length) > 0 {
        .response {status: 204}
        .append selection --meta {action: "select", id: $select_match.id.0} --ttl ephemeral | ignore
      } else {
        .response {status: 404}
        "Not Found"
      }
    }

    {method: "GET"} => {
      # Handle /asset/<hash>
      let asset_match = ($req.path | parse "/asset/{hash}")
      if ($asset_match | length) > 0 {
        let hash = $asset_match.hash.0
        .response {
          headers: {
            "Content-Type": "image/png"
            "Cache-Control": "public, max-age=31536000, immutable"
          }
        }
        .cas $hash
      } else {
        .static "www" $req.path
      }
    }
  }
}
