export alias "h. get" = h. request get
export alias "h. post" = h. request post

export def .cat [
    --follow
] {
    print "here"
    print $follow
    let postfix = if $follow { "//?follow" } else { "" }
    print $postfix
    h. get $"./store/sock($postfix)" | lines | each {|x| $x | from json}
}

export def .cas [hash?: string] {
    let hash = if ($hash | is-not-empty) { $hash } else { $in }
    h. get $"./store/sock//cas/($hash)"
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

