# Selection state management for pasteboard viewer

# Process selection action frames and maintain state
# State includes both items and selectedId to avoid closure variable issues
export def "process-selection" [
  items: list
  initial_state: record
] {

  generate {|frame, state|
    let action = ($frame.meta?.action? | default "")
    let current_id = $state.selectedId
    let items = $state.items

    let new_selected = match $action {
      "down" => {
        let current_index = ($items | enumerate | where {|it| $it.item.id == $current_id } | get index.0)
        let new_index = ($current_index + 1) mod ($items | length)
        $items | get $new_index | get id
      }
      "up" => {
        let current_index = ($items | enumerate | where {|it| $it.item.id == $current_id } | get index.0)
        let new_index = ($current_index - 1) mod ($items | length)
        $items | get $new_index | get id
      }
      "select" => {
        $frame.meta?.id? | default $current_id
      }
      _ => $current_id
    }

    let new_state = {selectedId: $new_selected items: $items}
    {
      out: {selectedId: $new_selected frameId: $frame.id}
      next: $new_state
    }
  } ($initial_state | insert items $items)
}

# Render state to HTML for SSE (internal - using built-in .mj)
export def "render-sse-internal" [
  items: list
  template_path: string
] {
  each {|state|
    let eventTimestamp = (.id unpack $state.frameId | get timestamp | into int | $in / 1_000_000)
    let data = {items: $items selectedId: $state.selectedId serverTimestamp: (date now | into int | $in / 1_000_000) eventTimestamp: $eventTimestamp}
    let section_html = ($data | .mj $template_path)
    let sse_data = ($section_html | lines | each {|line| $"data: elements ($line)" } | str join "\n")
    $"event: datastar-patch-elements\ndata: selector main\n($sse_data)\n\n"
  }
}

# Render state to HTML for SSE (external - using minijinja-cli)
export def "render-sse-external" [
  items: list
  template_path: string
] {
  each {|state|
    let eventTimestamp = (.id unpack $state.frameId | get timestamp | into int | $in / 1_000_000)
    let data = {items: $items selectedId: $state.selectedId serverTimestamp: (date now | into int | $in / 1_000_000) eventTimestamp: $eventTimestamp}
    let section_html = ($data | to json -r | minijinja-cli -f json $template_path -)
    let sse_data = ($section_html | lines | each {|line| $"data: elements ($line)" } | str join "\n")
    $"event: datastar-patch-elements\ndata: selector main\n($sse_data)\n\n"
  }
}

# Group pasteboard events by pb.recv base ID
export def "group-pasteboard-items" [] {
  use /Users/andy/s/03erbwsly19fej1b1zw5wv7r3/xs.nu *

  .cat
  | where topic == "pb.recv" or topic == "content"
  | last 31
  | generate {|frame, state = {}|
      if $frame.topic == "pb.recv" {
        $state | insert $frame.id [$frame] | {out: $in next: $in}
      } else if $frame.topic == "content" and ($frame.meta?.updates? | is-not-empty) {
        let base_id = $frame.meta.updates
        if ($state | get -o $base_id) != null {
          $state | update $base_id { prepend $frame } | {out: $in next: $in}
        } else {
          # Skip content events whose base pb.recv is not in our window
          {next: $state}
        }
      } else {
        {next: $state}
      }
    }
  | last
  | items {|k, v|
      let latest = $v.0
      let item_type = if $latest.topic == "pb.recv" {
        "raw"
      } else if $latest.topic == "content" and ($latest.meta?.content_type? | default "") == "image" {
        "image"
      } else {
        "text"
      }

      {
        base: $k
        id: $latest.id
        type: $item_type
        content: (if $item_type == "image" { null } else { .cas $latest.hash })
        hash: $latest.hash
        meta: ($latest.meta? | default {})
      }
    }
  | reverse
}
