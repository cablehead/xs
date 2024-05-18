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

export def h. [
    path: string
    --last-id: string
    --follow
] {
    let query = ( build-query { "last-id": $last_id, follow: $follow } )
    let url = $"localhost($path)($query)"
    print $url
    curl -sN --unix-socket ./store/sock $url | lines | each { from json }
}


def main [] {
    # let clip = ( h. / | first )
    # $clip.hash
    # h. $"/cas/($clip.hash)"
    print ( h. / )
    print ( h. / --last-id "123" --follow)
}
