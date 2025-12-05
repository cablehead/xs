def calc-resume-from [] {
  .head content | if ($in | is-not-empty) { get meta?.updates? } | default "head"
}

{
  resume_from: (calc-resume-from)

  run: {|frame|
    if $frame.topic != "pb.recv" { return }

    let data = .cas $frame.hash | from json | get types

    $data | get -o "public.png" | if ($in | is-not-empty) {
      $in | decode base64 | .append content --meta {
        updates: $frame.id
        content_type: "image"
      }
      return
    }

    $data | get -o "public.utf8-plain-text" | if ($in | is-not-empty) {
      $in | decode base64 | decode | .append content --meta {updates: $frame.id}
      return
    }

    $frame
  }
}
