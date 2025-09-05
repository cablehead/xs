use projection.nu *

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

export def __test_all [] {
  __test
  print ""
  __test_project_state
}
