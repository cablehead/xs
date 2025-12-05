{|frame state|
  let state = $state | default {}
  match $frame.topic {
    "pb.recv" => (
      $state | insert $frame.id [$frame] | {out: $in next: $in}
    )
    "content" => (
      $state | update $frame.meta.updates { prepend $frame } | {out: $in next: $in}
    )
    _ => {next: $state}
  }
}
