export alias "h. get" = h. request get
export alias "h. post" = h. request post

alias ? = if ($in | is-not-empty) {$in}
alias ?? = ? else { return }

def and-then [next: closure --else: closure] {
  if ($in | is-not-empty) {do $next} else {
    do $else
  }
}

export def store-addr [] {
  $env | get XS_ADDR? | ? else {"./store"}
}

# update to use (store-addr) and the xs cli
def _cat [options: record] {
  let params = [
    (if ($options | get follow? | default false) {"--follow"})
    (if ($options | get tail? | default false) {"--tail"})

    (if $options.last_id? != null {["--last-id" $options.last_id]})

    (if $options.limit? != null {["--limit" $options.limit]})
    (if $options.pulse? != null {["--pulse" $options.pulse]})
  ] | compact | flatten

  xs cat (store-addr) ...$params | lines | each {|x| $x | from json}
}

export def .cat [
  --follow (-f) # long poll for new events
  --pulse (-p): int # specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
  --tail (-t) # begin long after the end of the stream
  --last-id (-l): string
  --limit: int
] {
  _cat { follow: $follow pulse: $pulse tail: $tail last_id: $last_id limit: $limit }
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
  let hash = read_hash (if $hash != null {$hash} else {$alt})
  if $hash == null { return }
  xs cas (store-addr) $hash
}

export def .get [id: string] {
  xs get (store-addr) $id | from json
}

export def .head [
  topic: string # The topic to get the head frame for
  --follow (-f) # Follow the head frame for updates
] {
  let params = [
    (if $follow {"--follow"})
  ] | compact
  xs head (store-addr) $topic ...$params | lines | each {|x| $x | from json}
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
] {
  xs append (store-addr) $topic ...(
    [
      (if $meta != null {["--meta" ($meta | to json -r)]})
      (if $ttl != null {["--ttl" $ttl]})
    ] | compact | flatten
  ) | from json
}

export def .remove [id: string] {
  xs remove (store-addr) $id
}

export alias .rm = .remove

export def .export [path: string] {
  if ($path | path exists) {
    print "path exists"
    return
  }
  mkdir ($path | path join "cas")

  xs cat (store-addr) | save ($path | path join "frames.jsonl")

  open ($path | path join "frames.jsonl") | lines | each {from json | get hash} | uniq | each {|hash|
    let hash_64 = $hash | encode base64
    let out_path = $"($path)/cas/($hash_64)"
    print $out_path
    .cas $hash | save $out_path
  }
}

export def .import [path: string] {
  glob ([$path "cas"] | path join "*") | each {|x|
    let want = ($x | path basename | decode base64 | decode)
    let got = cat $x | xs cas-post (store-addr)
    if $got != $want {
      return (
        error make {
          msg: $"hash mismatch got=($got) want=($want)"
        }
      )
    }
    $got
  }

  open ($path | path join "frames.jsonl") | lines | each {xs import (store-addr)}
}

export def .test [] {
  use std assert ;
  let cases = [
    [
      "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
      "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
    ]
    [
      { hash: "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE=" }
      "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
    ]
    [null null]
    [{ goo: 123 } null]
  ]

  for case in $cases {
    assert equal (read_hash $case.0) $case.1
  }
}
