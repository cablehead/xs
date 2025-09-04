use /Users/andy/.config/nushell/modules/xs.nu *

def print [...rest] {
  if ($env | get -o DDEBUG) != null {
    $rest | do $env.DDEBUG
  }
}

def todos.update [frame: record] {
  let todos = $in
  match $frame.meta.action {
    "add" => ($todos | append {id: $frame.id text: $frame.meta.text completed: false})
    "toggle" => (
      $todos | each {|todo|
        if ($todo | columns | "id" in $in) and $todo.id == $frame.meta.id {
          {id: $todo.id text: $todo.text completed: (not $todo.completed)}
        } else {
          $todo
        }
      }
    )
    "delete" => ($todos | where id != $frame.meta.id)
    "toggle-all" => {
      let all_completed = ($todos | all {|todo| $todo.completed })
      $todos | each {|todo| {id: $todo.id text: $todo.text completed: (not $all_completed)} }
    }
    "clear-completed" => ($todos | where completed == false)
    _ => $todos
  }
}

def __test [] {
  print "Running update_todos tests..."

  # Test 1: Add action
  print "\n1. Testing 'add' action:"
  let todos = [{id: "1" text: "Test todo" completed: false}]
  let frame = {id: "2" meta: {action: "add" text: "New todo"}}
  let result = $todos | todos.update $frame
  print $"   Input todos: ($todos | length) items"
  print $"   Result todos: ($result | length) items"
  print $"   New todo added: ($result | where id == "2" | get text.0)"

  # Test 2: Toggle action (false to true)
  print "\n2. Testing 'toggle' action (false -> true):"
  let todos = [{id: "1" text: "Test todo" completed: false}]
  let frame = {meta: {action: "toggle" id: "1"}}
  let result = $todos | todos.update $frame
  print $"   Original completed: ($todos | where id == "1" | get completed.0)"
  print $"   New completed: ($result | where id == "1" | get completed.0)"

  # Test 3: Toggle action (true to false)
  print "\n3. Testing 'toggle' action (true -> false):"
  let todos = [{id: "1" text: "Test todo" completed: true}]
  let frame = {meta: {action: "toggle" id: "1"}}
  let result = $todos | todos.update $frame
  print $"   Original completed: ($todos | where id == "1" | get completed.0)"
  print $"   New completed: ($result | where id == "1" | get completed.0)"

  # Test 4: Delete action
  print "\n4. Testing 'delete' action:"
  let todos = [{id: "1" text: "Test todo" completed: false} {id: "2" text: "Another todo" completed: true}]
  let frame = {meta: {action: "delete" id: "1"}}
  let result = $todos | todos.update $frame
  print $"   Input todos: ($todos | length) items"
  print $"   Result todos: ($result | length) items"
  print $"   Remaining todo: ($result | get text.0)"

  # Test 5: Toggle-all action (some incomplete -> all complete)
  print "\n5. Testing 'toggle-all' action (some incomplete -> all complete):"
  let todos = [{id: "1" text: "Todo 1" completed: false} {id: "2" text: "Todo 2" completed: true}]
  let frame = {meta: {action: "toggle-all"}}
  let result = $todos | todos.update $frame
  let all_completed = ($result | all {|todo| $todo.completed })
  print $"   All todos now completed: ($all_completed)"

  # Test 6: Toggle-all action (all complete -> all incomplete)
  print "\n6. Testing 'toggle-all' action (all complete -> all incomplete):"
  let todos = [{id: "1" text: "Todo 1" completed: true} {id: "2" text: "Todo 2" completed: true}]
  let frame = {meta: {action: "toggle-all"}}
  let result = $todos | todos.update $frame
  let all_incomplete = ($result | all {|todo| not $todo.completed })
  print $"   All todos now incomplete: ($all_incomplete)"

  # Test 7: Clear-completed action
  print "\n7. Testing 'clear-completed' action:"
  let todos = [{id: "1" text: "Todo 1" completed: false} {id: "2" text: "Todo 2" completed: true} {id: "3" text: "Todo 3" completed: false}]
  let frame = {meta: {action: "clear-completed"}}
  let result = $todos | todos.update $frame
  print $"   Input todos: ($todos | length) items"
  let result_count = ($result | length)
  print $"   Result todos: ($result_count) items - completed todos removed"
  print $"   Remaining todos all incomplete: ($result | all {|todo| not $todo.completed })"

  # Test 8: Unknown action
  print "\n8. Testing unknown action:"
  let todos = [{id: "1" text: "Test todo" completed: false}]
  let frame = {meta: {action: "unknown"}}
  let result = $todos | todos.update $frame
  print $"   Input todos: ($todos | length) items"
  let result_count = ($result | length)
  print $"   Result todos: ($result_count) items - unchanged"
  print $"   Todos unchanged: (($todos | to json) == ($result | to json))"

  print "\n✅ All tests completed!"
}

def __test_project_state [] {
  print "Running project_todo_state tests..."

  # Create test fixture frames
  let frames = [
    {id: "1" topic: "todos" meta: {action: "add" text: "First todo"}}
    {id: "2" topic: "todos" meta: {action: "add" text: "Second todo"}}
    {topic: "todos" meta: {action: "toggle" id: "1"}}
    {topic: "xs.threshold"}
    {id: "3" topic: "todos" meta: {action: "add" text: "Third todo"}}
    {topic: "todos" meta: {action: "toggle" id: "2"}}
  ]

  print "\nTest fixture:"
  print "  - Add todo1, Add todo2, Toggle todo1, <threshold>, Add todo3, Toggle todo2"

  # Run actual project_todo_state function
  let outputs = ($frames | project_todo_state | where active_count? != null)

  let output_count = ($outputs | length)
  print $"\nGenerated ($output_count) outputs - expected: 3"

  # Test assertions
  if ($outputs | length) != 3 {
    print "❌ Expected exactly 3 outputs"
    return
  }

  # First output (at threshold): 1 active, 1 completed
  let output1 = $outputs.0
  print "\n1. At threshold output:"
  print $"   Todos: ($output1.todos | length), Active: ($output1.active_count), Completed: ($output1.completed_count)"
  if $output1.active_count != 1 or $output1.completed_count != 1 {
    print "❌ Expected active_count: 1, completed_count: 1"
    return
  }

  # Second output (after add todo3): 2 active, 1 completed
  let output2 = $outputs.1
  print "\n2. After add todo3 output:"
  print $"   Todos: ($output2.todos | length), Active: ($output2.active_count), Completed: ($output2.completed_count)"
  if $output2.active_count != 2 or $output2.completed_count != 1 {
    print "❌ Expected active_count: 2, completed_count: 1"
    return
  }

  # Third output (after toggle todo2): 1 active, 2 completed
  let output3 = $outputs.2
  print "\n3. After toggle todo2 output:"
  print $"   Todos: ($output3.todos | length), Active: ($output3.active_count), Completed: ($output3.completed_count)"
  if $output3.active_count != 1 or $output3.completed_count != 2 {
    print "❌ Expected active_count: 1, completed_count: 2"
    return
  }

  print "\n✅ All project_todo_state tests passed!"
}

def make_output_record [state: record] {
  let active_count = ($state.todos | where completed == false | length)
  let completed_count = ($state.todos | where completed == true | length)
  {
    todos: $state.todos
    active_count: $active_count
    completed_count: $completed_count
  }
}

def __test_all [] {
  __test
  print ""
  __test_project_state
}

def do_404 [req: record] {
  .response {status: 404}
  $"Not Found: ($req.method) ($req.path)"
}

def conditional-pipe [
  condition: bool
  action: closure
] {
  if $condition { do $action } else { }
}

def project_todo_state [] {
  generate {|frame, state = {todos: [] live: false}|
    if ($frame.topic == "xs.threshold") {
      return (
        $state | update live { true } | {next: $in out: (make_output_record $in)}
      )
    }

    if not ($frame.topic == "todos" and $frame.meta?.action? != null) {
      return {next: $state}
    }

    let state = $state | update todos { todos.update $frame }

    {next: $state} | conditional-pipe $state.live {
      insert out (make_output_record $state)
    }
  }
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
