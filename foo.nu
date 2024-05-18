#!/usr/bin/env -S nu

alias and-then = if ($in | is-not-empty)
alias ? = if ($in | is-not-empty) { $in }
alias ?? = ? else { return }

def h. [
    path: string
    --last-id: string
] {
    let query = ( $last_id | and-then { $"?( {last_id: $last_id} | url build-query)" }  )
    let url = $"localhost($path)($query)"
    curl -sN --unix-socket ./store/sock $url | lines | each { from json }
}

h. / --last-id "foo"

# let clip = ( h. / | first )
# $clip.hash
# h. $"/cas/($clip.hash)"
