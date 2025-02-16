export alias "h. get" = h. request get
export alias "h. post" = h. request post

export const XS_CONTEXT_SYSTEM = "0000000000000000000000000"

def and-then [next: closure --else: closure] {
  if ($in | is-not-empty) { do $next } else {
    do $else
  }
}

def or-else [or_else: closure] {
  if ($in | is-not-empty) { $in } else { do $or_else }
}

def conditional-pipe [
  condition: bool
  action: closure
] {
  if $condition { do $action } else { $in }
}

export def xs-addr [] {
  $env | get XS_ADDR? | or-else { "./store" }
}

export def xs-context [selected?: string] {
  $selected | if ($in | is-empty) { $env | get XS_CONTEXT? } else { }
}

# update to use (xs-addr) and the xs cli
def _cat [options: record] {
  let params = [
    (if ($options | get follow? | default false) { "--follow" })
    (if ($options | get tail? | default false) { "--tail" })

    (if $options.last_id? != null { ["--last-id" $options.last_id] })

    (if $options.limit? != null { ["--limit" $options.limit] })
    (if $options.pulse? != null { ["--pulse" $options.pulse] })
    (if $options.context? != null { ["--context" $options.context] })
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
] {
  _cat {
    follow: $follow
    pulse: $pulse
    tail: $tail
    last_id: $last_id
    limit: $limit
    context: (xs-context $context)
  } | conditional-pipe (not $detail) { reject context_id ttl }
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
  topic: string # The topic to get the head frame for
  --follow (-f) # Follow the head frame for updates
] {
  if $follow {
    xs head (xs-addr) $topic --follow | lines | each {|x| $x | from json }
  } else {
    xs head (xs-addr) $topic | from json
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
      (xs-context $context | and-then { ["--context" $in] })
    ] | compact | flatten
  ) | from json
}

export def .remove [id: string] {
  xs remove (xs-addr) $id
}

export alias .rm = .remove

export def ".ctx" [] {
  xs-context | or-else { $XS_CONTEXT_SYSTEM }
}

export def ".ctx list" [] {
  let active = .ctx
  .cat -c $XS_CONTEXT_SYSTEM | where topic == "xs.context" | get id | prepend $XS_CONTEXT_SYSTEM | each {|x|
    {id: $x active: ($x == $active)}
  }
}

export def --env ".ctx switch" [id: string] {
  $env.XS_CONTEXT = $id
  .ctx
}

export def --env ".ctx new" [] {
  .append "xs.context" -c $XS_CONTEXT_SYSTEM | .ctx switch $in.id
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

  open ($path | path join "frames.jsonl") | lines | each { xs import (xs-addr) }
}
