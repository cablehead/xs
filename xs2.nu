export alias "h. get" = h. request get
export alias "h. post" = h. request post

alias and-then = if ($in | is-not-empty)
alias ? = if ($in | is-not-empty) { $in }
alias ?? = ? else { return }


def _cat [options: record] {
    let params = [
        (if ($options | get follow? | default false) {
            "follow" + (if $options.pulse? != null { $"=($options.pulse)" } else { "" })
        })

        (if ($options | get tail? | default false) { "tail" })

        (if ($options.last_id? | is-not-empty) { $"last-id=($options.last_id)" })
    ] | compact

    let postfix = if ($params | is-not-empty) {
        "//?" + ($params | str join "&")
    } else { "" }

    h. get $"./store/sock($postfix)" | lines | each { |x| $x | from json }
}

export def .cat [
    --follow (-f)       # long poll for new events
    --pulse (-p): int   # specifies the interval (in milliseconds) to receive a synthetic "xs.pulse" event
    --tail (-t)         # begin long after the end of the stream
    --last-id (-l): string
] {
    _cat {follow: $follow pulse: $pulse tail: $tail last_id: $last_id}
}

def read_hash [hash?: any] {
    match ($hash | describe -d | get type) {
        "string" => $hash
        "record" => ($hash | get hash?)
        _ => null
    }
}

export def .step [
    handler: closure
    meta_path: string
    --follow (-f)       # long poll for new events
] {
    loop {
        let meta = try { open -r $meta_path } catch { "{}" } | from json
        let frame = _cat ($meta | insert follow $follow)  | try { first } catch { return }
        let res = do $handler {} $frame
        if $res == null {
            {last_id: $frame.id} | to json -r | save -rf $meta_path
            continue
        }

        return $res
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
    # TODO: this `null | `is an issue with the plugin h.'s api:: to resolve
    # submit a PR to add unix socket support to Nushell's built-in HTTP
    # commands
    null | h. get $"./store/sock//($id)" | from json
}

export def .head [topic: string] {
    null | h. get $"./store/sock//head/($topic)" | from json
}

export def .append [topic: string --meta: record] {
    h. post $"./store/sock//($topic)" --headers {"xs-meta": ($meta | to json -r)}
}

export def .pipe [id: string] {
    let sp = (metadata $in).span
    let script = $in
    let content = match ($script | describe -d | get type) {
        "string" => $script
        "closure" => {view source $script}
        _ => {return (error make {
            msg: "script should either be a string or closure"
            label: {
                text: "script input"
                span: $sp
            }
        })}
    }
    $content | h. post $"./store/sock//pipe/($id)"
}

# show the status of running tasks TBD
export def .tasks [] {
    .cat
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
