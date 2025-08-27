export alias "h. get" = h. request get
export alias "h. post" = h. request post

export const XS_CONTEXT_SYSTEM = "0000000000000000000000000"

def and-then [next: closure --else: closure] {
  if ($in | is-not-empty) { do $next } else {
    if $else != null { do $else }
  }
}

def or-else [or_else: closure] {
  if ($in | is-not-empty) { $in } else { do $or_else }
}

def conditional-pipe [
  condition: bool
  action: closure
] {
  if $condition { do $action } else { }
}

export def xs-addr [] {
  $env | get XS_ADDR? | or-else { try { open ~/.config/cross.stream/XS_ADDR | str trim | path expand } } | or-else { "~/.local/share/cross.stream/store" | path expand }
}

export def xs-context-collect [] {
  _cat {context: $XS_CONTEXT_SYSTEM} | reduce --fold {} {|frame acc|
    match $frame.topic {
      "xs.context" => ($acc | insert $frame.id $frame.meta?.name?)
      "xs.annotate" => (
        if $frame.meta?.updates? in $acc {
          $acc | update $frame.meta.updates $frame.meta?.name?
        } else {
          $acc
        }
      )
      _ => $acc
    }
  } | transpose id name | prepend {
    id: $XS_CONTEXT_SYSTEM
    name: "system"
  }
}

export def xs-context [selected?: string span?] {
  if $selected == null {
    return ($env | get XS_CONTEXT?)
  }

  xs-context-collect | where id == $selected or name == $selected | try { first | get id } catch {
    if $span != null {
      error make {
        msg: $"context not found: ($selected)"
        label: {text: "provided span" span: $span}
      }
    } else {
      error make -u {msg: $"context not found: ($selected)"}
    }
  }
}

def _cat [options: record] {
  let params = [
    (if ($options | get follow? | default false) { "--follow" })
    (if ($options | get tail? | default false) { "--tail" })
    (if ($options | get all? | default false) { "--all" })

    (if $options.last_id? != null { ["--last-id" $options.last_id] })

    (if $options.limit? != null { ["--limit" $options.limit] })
    (if $options.pulse? != null { ["--pulse" $options.pulse] })
    (if $options.context? != null { ["--context" $options.context] })
    (if $options.topic? != null { ["--topic" $options.topic] })
  ] | compact | flatten

  xs cat (xs-addr) ...$params | lines | each {|x| $x | from json }
}

export def .cat [
  --follow (-f) # long poll for new events
  --pulse (-p): int # specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
  --tail (-t) # begin long after the end of the stream
  --detail (-d) # include all frame fields in the output
  --last-id (-l): string
  --limit: int
  --context (-c): string # the context to read from
  --all (-a) # cat across all contexts
  --topic (-T): string # filter by topic
] {
  _cat {
    follow: $follow
    pulse: $pulse
    tail: $tail
    last_id: $last_id
    limit: $limit
    context: (if not $all { (xs-context $context (metadata $context).span) })
    all: $all
    topic: $topic
  } | conditional-pipe (not ($detail or $all)) { each { reject context_id ttl } }
}

def read_hash [hash?: any] {
  match ($hash | describe -d | get type) {
    "string" => $hash
    "record" => ($hash | get hash?)
    _ => null
  }
}

export def .cas [hash?: any] {
  let alt = $in
  let hash = read_hash (if $hash != null { $hash } else { $alt })
  if $hash == null { return }
  xs cas (xs-addr) $hash
}

export def .get [id: string] {
  xs get (xs-addr) $id | from json
}

export def .head [
  topic: string
  --follow (-f)
  --context (-c): string
] {
  let params = [
    (xs-context $context (metadata $context).span | and-then { ["--context" $in] })
  ] | compact | flatten

  if $follow {
    xs head (xs-addr) $topic ...($params) --follow | lines | each {|x| $x | from json }
  } else {
    xs head (xs-addr) $topic ...($params) | from json
  }
}

# Append an event to the stream
export def .append [
  topic: string # The topic to append the event to
  --meta: record # Optional metadata to include with the event, provided as a record
  --context (-c): string # the context to append to
  --ttl: string # Optional Time-To-Live for the event. Supported formats:
  #   - "forever": The event is kept indefinitely.
  #   - "ephemeral": The event is not stored; only active subscribers can see it.
  #   - "time:<milliseconds>": The event is kept for a custom duration in milliseconds.
  #   - "head:<n>": Retains only the last n events for the topic (n must be >= 1).
] {
  xs append (xs-addr) $topic ...(
    [
      (if $meta != null { ["--meta" ($meta | to json -r)] })
      (if $ttl != null { ["--ttl" $ttl] })
      (xs-context $context (metadata $context).span | and-then { ["--context" $in] })
    ] | compact | flatten
  ) | from json
}

export def .remove [id: string] {
  xs remove (xs-addr) $id
}

export alias .rm = .remove

export def ".ctx" [
  --detail (-d) # return a record with id and name fields
] {
  let id = xs-context | or-else { $XS_CONTEXT_SYSTEM }
  let name = xs-context-collect | where id == $id | get name.0
  if $detail {
    {id: $id} | if $name != null { insert name $name } else { $in }
  } else {
    $name | default $id
  }
}

export def ".ctx list" [] {
  let active = .ctx -d | get id
  xs-context-collect | insert active {
    $in.id == $active
  }
}

export alias ".ctx ls" = .ctx list

export def --env ".ctx switch" [id?: string] {
  $env.XS_CONTEXT = $id | or-else { .ctx select }
  .ctx --detail | get id
}

export def --env ".ctx new" [name: string] {
  .append "xs.context" -c $XS_CONTEXT_SYSTEM --meta {name: $name} | .ctx switch $in.id
}

export def --env ".ctx rename" [id: string name: string] {
  .append "xs.annotate" -c $XS_CONTEXT_SYSTEM --meta {
    updates: (xs-context $id (metadata $id).span)
    name: $name
  }
}

export def --env ".ctx select" [] {
  .ctx list | input list | get id
}

export def .export [path: string] {
  if ($path | path exists) {
    print "path exists"
    return
  }
  mkdir ($path | path join "cas")

  xs cat (xs-addr) | save ($path | path join "frames.jsonl")

  open ($path | path join "frames.jsonl") | lines | each { from json | get hash } | uniq | each {|hash|
    let hash_64 = $hash | encode base64
    let out_path = $"($path)/cas/($hash_64)"
    print $out_path
    .cas $hash | save $out_path
  }
}

export def .import [path: string] {
  glob ([$path "cas"] | path join "*") | each {|x|
    let want = ($x | path basename | decode base64 | decode)
    let got = cat $x | xs cas-post (xs-addr)
    if $got != $want {
      return (
        error make {
          msg: $"hash mismatch got=($got) want=($want)"
        }
      )
    }
    $got
  }

  open ($path | path join "frames.jsonl") | lines | each {
    from json | default "0000000000000000000000000" context_id | to json -r | xs import (xs-addr)
  }
}

# Execute a Nushell script with store helper commands available
export def .exec [script?: string] {
  let input_script = if $script != null { $script } else { $in }
  if $input_script == null {
    error make {msg: "No script provided as argument or via pipeline"}
  }
  xs exec (xs-addr) $input_script
}

# Generate a new SCRU128 ID
export def .id [] {
  xs scru128
}

# Unpack a SCRU128 ID into its component fields
export def ".id unpack" [id?: string] {
  let input_id = if $id != null { $id } else { $in }
  if $input_id == null {
    error make {msg: "No ID provided as argument or via pipeline"}
  }

  let components = xs scru128 unpack $input_id | from json
  $components | update timestamp ($components.timestamp * 1000000000 | into int | into datetime)
}

# Pack component fields into a SCRU128 ID
export def ".id pack" [components?: record] {
  let input_components = if $components != null { $components } else { $in }
  if $input_components == null {
    error make {msg: "No components provided as argument or via pipeline"}
  }

  $input_components
  | conditional-pipe (($input_components.timestamp | describe) == "datetime") {
    update timestamp ($input_components.timestamp | into int | $in / 1000000000)
  }
  | to json
  | xs scru128 pack
}

# Spawn xs serve in a temporary directory, run a closure, then cleanup
export def .tmp-spawn [
  closure: closure
  --interactive (-i) # Start an interactive nu shell after running the closure
] {
  # Create a temporary directory
  let tmp_dir = (mktemp -d)
  print $"Created temp directory: ($tmp_dir)"

  let store_path = ($tmp_dir | path join "store")

  try {
    # Create store directory
    mkdir $store_path

    # Spawn xs serve in the background
    let job_id = job spawn --tag "xs-test-server" {
      xs serve $store_path
    }
    print $"Started xs serve with job ID: ($job_id)"

    $env.XS_ADDR = $store_path
    $env.XS_CONTEXT = null

    # Give the server a moment to start up
    sleep 500ms

    try {
      # Run the provided closure
      do $closure
    } catch {|err|
      error make {msg: $"Error in closure: ($err.msg)"}
    }

    # Start interactive nu shell if requested
    if $interactive {
      nu
    }

    # Kill the background job
    job kill $job_id
    print $"Killed xs serve job ($job_id)"

    # Give a moment for the job to shut down
    sleep 50ms
  } catch {|err|
    error make {msg: $"Error during setup: ($err.msg)"}
  }

  # Clean up the temporary directory
  try {
    # rm -rf $tmp_dir
    print $"Cleaned up temp directory: ($tmp_dir)"
  } catch {|err|
    print $"Could not clean up temp directory: ($err.msg)"
  }
}
