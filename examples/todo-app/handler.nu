{
  process: {|frame|
    if $frame.topic != "http.request" { return }

    match $frame.meta {
      {uri: "/" method: "GET"} => {
        # Get todos
        let todos = .cat | where topic =~ "todo" | reduce --fold {} {|f acc|
          match $f.topic {
            "todo" => ($acc | insert $f.id {text: (.cas $f.hash) done: false})
            todo.toggle => ($acc | update ($f.meta.id) { $in | update done { not ($in) } })
            _ => $acc
          }
        }

        # Render template
        {todos: $todos} | to json -r | minijinja-cli -f json -t (.head index.html | .cas $in.hash) '' - | .append http.response --meta {
          request_id: $frame.id
          status: 200
          headers: {
            Content-Type: "text/html"
          }
        }
      }

      {uri: "/" method: "POST"} => {
        # Get the todo content and store it
        .cas $frame.hash | url split-query | transpose -rdl | get todo | .append todo

        # Redirect to GET /
        .append http.response --meta {
          request_id: $frame.id
          status: 303
          headers: {
            Location: "/"
          }
        }
      }

      {uri: "/toggle" method: "POST"} => {
        .cas $frame.hash | from json | .append todo.toggle --meta $in
        "OK" | .append http.response --meta {request_id: $frame.id status: 200 headers: {Content-Type: "text/plain"}}
      }

      _ => {
        "not found :/" | .append http.response --meta {
          request_id: $frame.id
          status: 404
          headers: {
            Content-Type: "text/html"
          }
        }
      }
    }
  }
}
