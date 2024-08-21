export alias "h. get" = h. request get
export alias "h. post" = h. request post

alias and-then = if ($in | is-not-empty)
alias ? = if ($in | is-not-empty) { $in }
alias ?? = ? else { return }

export def .cat [
    --follow (-f)
] {
    let postfix = if $follow { "//?follow" } else { "" }
    h. get $"./store/sock($postfix)" | lines | each { |x| $x | from json }
}

export def .cas [hash?: string] {
    $hash | and-then {
        let uri = $"./store/sock//cas/($hash)"
        h. get $uri
    }
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

