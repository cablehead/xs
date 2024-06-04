alias and-then = if ($in | is-not-empty)
alias ? = if ($in | is-not-empty) { $in }
alias ?? = ? else { return }

def build-query [params] {
    $params | columns | each { |x|
        let value = ($params | get $x)
        match ( $value | describe ) {
            "string" => $"($x)=($value)",
            "bool" => (if $value { $x }),
        }
    } | and-then { $"?($in | str join "&")" }
}

def flatten-params [params] {
    $params | columns | each {|name|
        $params | get $name | and-then {
            let value = $in
            if $value == true {
                [$name]
            } else {
                [$name, $value]
            }

        }
    } | flatten
}

export def cat [
    store: string
    --last-id: any
    --follow
    --tail
] {
    let path = "/"
    let query = ( build-query { "last-id": $last_id, follow: $follow, tail: $tail } )
    let url = $"localhost($path)($query)"
    curl -sN --unix-socket $"($store)/sock" $url | lines | each { from json }
}

export def process [
    store: string
    callback: closure
] {
    each {|meta| cas $store $meta.hash | do $callback $meta}
}

export def stream-get [
    store: string
    id: string
] {
    let url = $"localhost/($id)"
    curl -sN --unix-socket $"($store)/sock" $url | from json
}

export def append [
    store: string
    topic: string
    --meta: record
] {
    curl -s -T - -X POST ...(
        $meta | and-then {
            ["-H" $"xs-meta: ($meta | to json -r)"]
        } | default []
    ) --unix-socket $"($store)/sock" $"localhost(if ($topic | str starts-with '/') { $topic } else { $"/($topic)" })"
}

export def cas [
    store: string
    hash: string
] {
    curl -sN --unix-socket $"($store)/sock" $"localhost/cas/($hash)"
}
