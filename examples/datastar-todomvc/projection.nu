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
    interaction_count: $state.interaction_count
    time_to_first_view: $state.time_to_first_view
  }
}

def conditional-pipe [
  condition: bool
  action: closure
] {
  if $condition { do $action } else { }
}

export def project_todo_state [] {
  generate {|frame, state = {todos: [] live: false interaction_count: 0 start_time: null time_to_first_view: null}|
    if ($frame.topic == "xs.threshold") {
      let current_time = (date now | format date "%s%3f" | into float)
      let time_to_first_view = if $state.start_time != null {
        let ms = ($current_time - $state.start_time) | math round | into int
        if $ms < 1 { 1 } else { $ms }
      } else {
        null
      }
      return (
        $state | update live { true } | update time_to_first_view { $time_to_first_view } | {next: $in out: (make_output_record $in)}
      )
    }

    if not ($frame.topic == "todos" and $frame.meta?.action? != null) {
      return {next: $state}
    }

    # Record start time on first todo action
    let state = if $state.start_time == null {
      $state | update start_time { date now | format date "%s%3f" | into float }
    } else {
      $state
    }

    let state = $state | update todos { todos.update $frame } | update interaction_count { $in + 1 }

    {next: $state} | conditional-pipe $state.live {
      insert out (make_output_record $state)
    }
  }
}
