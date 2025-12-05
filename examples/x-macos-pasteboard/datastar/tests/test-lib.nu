#!/usr/bin/env nu

use std/assert
use ../lib *

# Test data
let test_items = [
  {id: "item-1", type: "content", content: "First"}
  {id: "item-2", type: "content", content: "Second"}
  {id: "item-3", type: "content", content: "Third"}
  {id: "item-4", type: "content", content: "Fourth"}
]

# Helper to create selection frames
def make-frame [action: string, id?: string] {
  if $id != null {
    {meta: {action: $action, id: $id}}
  } else {
    {meta: {action: $action}}
  }
}

# Test: Initial state with down action
def test_down_from_first [] {
  let result = (
    [(make-frame "down")]
    | process-selection $test_items {selectedId: "item-1"}
    | first
  )

  assert equal $result.selectedId "item-2" "Down from first should select second"
}

# Test: Down from last item wraps to first
def test_down_wrap_around [] {
  let result = (
    [(make-frame "down")]
    | process-selection $test_items {selectedId: "item-4"}
    | first
  )

  assert equal $result.selectedId "item-1" "Down from last should wrap to first"
}

# Test: Up from first item wraps to last
def test_up_wrap_around [] {
  let result = (
    [(make-frame "up")]
    | process-selection $test_items {selectedId: "item-1"}
    | first
  )

  assert equal $result.selectedId "item-4" "Up from first should wrap to last"
}

# Test: Up action moves to previous item
def test_up_from_third [] {
  let result = (
    [(make-frame "up")]
    | process-selection $test_items {selectedId: "item-3"}
    | first
  )

  assert equal $result.selectedId "item-2" "Up from third should select second"
}

# Test: Select action sets specific item
def test_select_specific_item [] {
  let result = (
    [(make-frame "select" "item-3")]
    | process-selection $test_items {selectedId: "item-1"}
    | first
  )

  assert equal $result.selectedId "item-3" "Select should set specific item"
}

# Test: Multiple navigation actions in sequence
def test_multiple_actions [] {
  let results = (
    [
      (make-frame "down")     # item-1 -> item-2
      (make-frame "down")     # item-2 -> item-3
      (make-frame "up")       # item-3 -> item-2
      (make-frame "select" "item-4")  # jump to item-4
      (make-frame "down")     # item-4 -> item-1 (wrap)
    ]
    | process-selection $test_items {selectedId: "item-1"}
    | each {|state| $state.selectedId}
  )

  assert equal ($results | get 0) "item-2" "First down"
  assert equal ($results | get 1) "item-3" "Second down"
  assert equal ($results | get 2) "item-2" "Up"
  assert equal ($results | get 3) "item-4" "Select"
  assert equal ($results | get 4) "item-1" "Down with wrap"
}

# Test: Unknown action preserves state
def test_unknown_action_preserves_state [] {
  let result = (
    [(make-frame "unknown")]
    | process-selection $test_items {selectedId: "item-2"}
    | first
  )

  assert equal $result.selectedId "item-2" "Unknown action should preserve state"
}

# Run all tests
def main [] {
  print "Running selection navigation tests...\n"

  try {
    test_down_from_first
    print "✓ Down from first"
  } catch {|e|
    print $"✗ Down from first: ($e.msg)"
  }

  try {
    test_down_wrap_around
    print "✓ Down wrap around"
  } catch {|e|
    print $"✗ Down wrap around: ($e.msg)"
  }

  try {
    test_up_wrap_around
    print "✓ Up wrap around"
  } catch {|e|
    print $"✗ Up wrap around: ($e.msg)"
  }

  try {
    test_up_from_third
    print "✓ Up from third"
  } catch {|e|
    print $"✗ Up from third: ($e.msg)"
  }

  try {
    test_select_specific_item
    print "✓ Select specific item"
  } catch {|e|
    print $"✗ Select specific item: ($e.msg)"
  }

  try {
    test_multiple_actions
    print "✓ Multiple actions sequence"
  } catch {|e|
    print $"✗ Multiple actions sequence: ($e.msg)"
  }

  try {
    test_unknown_action_preserves_state
    print "✓ Unknown action preserves state"
  } catch {|e|
    print $"✗ Unknown action preserves state: ($e.msg)"
  }

  print "\nAll tests completed!"
}
