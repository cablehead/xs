{||
    let frame = $in
    if $frame.topic != "pb.recv" { return }

    let data = .cas $frame.hash | from json | get types

    $data | get "public.png"? | if ($in | is-not-empty) {
        $in | decode base64 | .append content --meta {
            updates: $frame.id
            content_type: "image" }
        return
    }

    $data | get "public.utf8-plain-text"? | if ($in | is-not-empty) {
        $in | decode base64 | decode | .append content --meta {updates: $frame.id}
        return
    }

    $frame
}
