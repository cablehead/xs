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
  if $condition { do $action } else { $in }
}

export def xs-addr [] {
  $env | get XS_ADDR? | or-else { "./store" }
}

export def xs-context-collect [] {
  _cat {context: $XS_CONTEXT_SYSTEM} | where topic == "xs.context" | each {
    {
      id: $in.id
      name: $in.meta?.name?
    }
  } | prepend {
    id: $XS_CONTEXT_SYSTEM
    name: "system"
  }
}

export def xs-context [selected?: string] {

  if $selected == null {
    return $env | get XS_CONTEXT?
  }

  let span = (metadata $selected).span;

  xs-context-collect | where id == $selected or name == $selected | try { first | get id } catch {
    error make {
      msg: $"context not found: ($selected)"
      label: {text: "provided context" span: $span}
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
] {
  _cat {
    follow: $follow
    pulse: $pulse
    tail: $tail
    last_id: $last_id
    limit: $limit
    context: (if not $all { (xs-context $context) })
    all: $all
  } | conditional-pipe (not ($detail or $all)) { reject context_id ttl }
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
    (xs-context $context | and-then { ["--context" $in] })
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
  xs-context-collect | insert active {
    $in.id == $active
  }
}

export alias ".ctx ls" = .ctx list

export def --env ".ctx switch" [id?: string] {
  $env.XS_CONTEXT = $id | or-else { .ctx select }
  .ctx
}

export def --env ".ctx new" [name: string] {
  .append "xs.context" -c $XS_CONTEXT_SYSTEM --meta {name: $name} | .ctx switch $in.id
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
