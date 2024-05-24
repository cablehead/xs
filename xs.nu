#!/usr/bin/env -S nu

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

export def cat [
    store: string
    --last-id: string
    --follow
] {
    let path = "/"
    let query = ( build-query { "last-id": $last_id, follow: $follow } )
    let url = $"localhost($path)($query)"
    curl -sN --unix-socket $"($store)/sock" $url | lines | each { from json }
}

export def append [
    store: string
    topic: string
] {
    curl -s -T - -X POST --unix-socket $"($store)/sock" $"localhost($topic)"
}

export def cas [
    store: string
    hash: string
] {
    curl -sN --unix-socket $"($store)/sock" $"localhost/cas/($hash)"
}


def main [] {
    # let clip = ( h. / | first )
    # $clip.hash
    # h. $"/cas/($clip.hash)"
    print ( cat ./store )
    print ( cat ./store --last-id "123" --follow)
}
