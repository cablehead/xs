export alias "h. get" = h. request get
export alias "h. post" = h. request post

alias and-then = if ($in | is-not-empty)
alias ? = if ($in | is-not-empty) { $in }
alias ?? = ? else { return }

export def .cat [
    --follow (-f)       # long poll for new events
    --pulse (-p): int   # specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
    --tail (-t)         # begin long after the end of the stream
    --last-id (-l): string
] {
    let params = [
        (if $follow {
            "follow" + (if $pulse != null { $"=($pulse)" } else { "" })
        })

        (if $tail { "tail" })

        (if ($last_id | is-not-empty) { $"last-id=($last_id)" })
    ] | compact

    let postfix = if ($params | is-not-empty) {
        "//?" + ($params | str join "&")
    } else { "" }

    h. get $"./store/sock($postfix)" | lines | each { |x| $x | from json }
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
    let uri = $"./store/sock//cas/($hash)"
    h. get $uri
}

export def .get [id: string] {
    h. get $"./store/sock//($id)" | from json
}

export def .append [topic: string --meta: record] {
    h. post $"./store/sock//($topic)" --headers {"xs-meta": ($meta | to json -r)}
}

export def .pipe [id: string snippet: closure] {
    view source $snippet | h. post $"./store/sock//pipe/($id)"
}


export def .test [] {
    use std assert;
    let cases = [
        [
            "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
            "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
        ]
        [
            {hash: "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="}
            "sha256-k//MXqRXKqeE+7S7SkKSbpU3dWrxwzh/iR6v683XTyE="
        ]
        [ null null ]
        [ {goo: 123} null ]
    ]

    for case in $cases {
        assert equal (read_hash $case.0) $case.1
    }
}
