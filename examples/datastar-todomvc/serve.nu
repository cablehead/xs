use /Users/andy/.config/nushell/modules/xs.nu *
use projection.nu *

def do_404 [req: record] {
  .response {status: 404}
  $"Not Found: ($req.method) ($req.path)"
}

{|req|
  let body = $in

  match $req {
    {method: "GET" , path: "/todos/updates"} => {
      .response {
        headers: {
          "Content-Type": "text/event-stream"
          "Cache-Control": "no-cache"
        }
      }
      .cat -T todos -f | project_todo_state | each {|state|
        let section_html = ($state | to json -r | minijinja-cli -f json templates/todo_section.html -)
        let sse_data = ($section_html | lines | each {|line| $"data: elements ($line)" } | str join "\n")
        $"event: datastar-patch-elements\n($sse_data)\n\n"
      }
    }
    {method: "POST" , path: "/todos/add"} => {
      let text = $body | from json | get input | str trim
      if ($text | is-empty) {
        .response {status: 400}
        "Empty todo text"
      } else {
        .response {status: 204}
        .append todos --meta {action: "add" text: $text} | ignore
      }
    }
    {method: "POST" , path: "/todos/toggle-all"} => {
      .response {status: 204}
      .append todos --meta {action: "toggle-all"} | ignore
    }
    {method: "DELETE" , path: "/todos/completed"} => {
      .response {status: 204}
      .append todos --meta {action: "clear-completed"} | ignore
    }
    {method: "POST"} => {
      let toggle_match = $req.path | parse "/todos/{id}/toggle"
      if ($toggle_match | length) > 0 {
        let todo_id = $toggle_match.id.0
        .response {status: 204}
        .append todos --meta {action: "toggle" id: $todo_id} | ignore
      } else {
        do_404 $req
      }
    }
    {method: "DELETE"} => {
      let delete_match = $req.path | parse "/todos/{id}"
      if ($delete_match | length) > 0 {
        let todo_id = $delete_match.id.0
        .response {status: 204}
        .append todos --meta {action: "delete" id: $todo_id} | ignore
      } else {
        do_404 $req
      }
    }
    {method: "GET"} => {
      .static "./www" $req.path --fallback "index.html"
    }
    _ => {
      .response {status: 405}
      "Method not allowed"
    }
  }
}
