export alias "h. get" = h. request get
export alias "h. post" = h. request post

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
  $env | get XS_ADDR? | or-else { "~/.local/share/cross.stream/store" | path expand }
}

def _cat [options: record] {
  let with_ts = ($options | get with_timestamp? | default false)
  let params = [
    (if ($options | get follow? | default false) { "--follow" })
    (if ($options | get new? | default false) { "--new" })
    (if $with_ts { "--with-timestamp" })

    (if $options.after? != null { ["--after" $options.after] })
    (if $options.from? != null { ["--from" $options.from] })

    (if $options.limit? != null { ["--limit" $options.limit] })
    (if $options.last? != null { ["--last" $options.last] })
    (if $options.pulse? != null { ["--pulse" $options.pulse] })
    (if $options.topic? != null { ["--topic" $options.topic] })
  ] | compact | flatten

  xs cat (xs-addr) ...$params | lines | each {|x|
    $x | from json | if $with_ts { into datetime timestamp } else { }
  }
}

export def .cat [
  --follow (-f) # long poll for new events
  --pulse (-p): int # specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
  --new (-n) # skip existing, only show new
  --detail (-d) # include all frame fields in the output
  --with-timestamp # include RFC3339 timestamp extracted from frame ID
  --after: string # start after a specific frame ID (exclusive)
  --from: string # start from a specific frame ID (inclusive)
  --limit: int
  --last: int # return the last N events (most recent)
  --topic (-T): string # filter by topic
] {
  _cat {
    follow: $follow
    pulse: $pulse
    new: $new
    with_timestamp: $with_timestamp
    after: $after
    from: $from
    limit: $limit
    last: $last
    topic: $topic
  } | conditional-pipe (not $detail) { each { reject ttl } }
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

export def .cas-post [] {
  $in | xs cas-post (xs-addr)
}

export def .get [
  id: string
  --with-timestamp # include RFC3339 timestamp extracted from frame ID
] {
  xs get (xs-addr) $id ...(if $with_timestamp { ["--with-timestamp"] } else { [] })
  | from json
  | if $with_timestamp { into datetime timestamp } else { }
}

export def .last [
  topic?: string
  --last (-n): int  # Number of frames to return
  --follow (-f)
  --with-timestamp # include RFC3339 timestamp extracted from frame ID
] {
  let args = [
    (if $topic != null { [$topic] })
    (if $last != null { ["-n" $last] })
    (if $follow { ["--follow"] })
    (if $with_timestamp { ["--with-timestamp"] })
  ] | compact | flatten

  if $follow or ($last != null and $last > 1) {
    xs last (xs-addr) ...$args | lines | each {|x|
      $x | from json | if $with_timestamp { into datetime timestamp } else { }
    }
  } else {
    xs last (xs-addr) ...$args | from json | if $with_timestamp { into datetime timestamp } else { }
  }
}

# Append an event to the stream
export def .append [
  topic: string # The topic to append the event to
  --meta: record # Optional metadata to include with the event, provided as a record
  --ttl: string # Optional Time-To-Live for the event. Supported formats:
  #   - "forever": The event is kept indefinitely.
  #   - "ephemeral": The event is not stored; only active subscribers can see it.
  #   - "time:<milliseconds>": The event is kept for a custom duration in milliseconds.
  #   - "head:<n>": Retains only the last n events for the topic (n must be >= 1).
  --with-timestamp # include RFC3339 timestamp extracted from frame ID
] {
  xs append (xs-addr) $topic ...(
    [
      (if $meta != null { ["--meta" ($meta | to json -r)] })
      (if $ttl != null { ["--ttl" $ttl] })
      (if $with_timestamp { ["--with-timestamp"] })
    ] | compact | flatten
  ) | from json | if $with_timestamp { into datetime timestamp } else { }
}

export def .remove [id: string] {
  xs remove (xs-addr) $id
}

export alias .rm = .remove

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
    from json | to json -r | xs import (xs-addr)
  }
}

# Evaluate a Nushell script with store helper commands available
export def .eval [
  file?: string             # Script file to evaluate, or "-" for stdin
  --commands (-c): string   # Evaluate script from command line
] {
  if $commands != null {
    xs eval (xs-addr) -c $commands
  } else if $file != null {
    xs eval (xs-addr) $file
  } else {
    let input_script = $in
    if $input_script == null {
      error make {
        msg: "No script provided"
        help: "Provide a file (.eval script.nu), use -c (.eval -c '<script>'), or pipe input ('<script>' | .eval)"
      }
    }
    xs eval (xs-addr) -c $input_script
  }
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
