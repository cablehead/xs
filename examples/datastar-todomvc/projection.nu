export def todos.update [frame: record] {
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

export def make_output_record [state: record] {
  let active_count = ($state.todos | where completed == false | length)
  let completed_count = ($state.todos | where completed == true | length)
  {
    todos: $state.todos
    active_count: $active_count
    completed_count: $completed_count
  }
}

def conditional-pipe [
  condition: bool
  action: closure
] {
  if $condition { do $action } else { }
}

export def project_todo_state [] {
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
