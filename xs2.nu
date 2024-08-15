export alias "h. get" = h. request get
export alias "h. post" = h. request post

export def .cat [] {
    h. get ./store/sock | lines | each {from json}
}

export def .cas [hash: string] {
    h. get $"./store/sock//cas/($hash)"
}

export def .get [id: string] {
    h. get $"./store/sock//($id)" | from json
}

export def .append [topic: string --meta: record] {
    h. post $"./store/sock//($topic)" --headers {"xs-meta": ($meta | to json -r)}
}

export def .pipe [id: string] {
    h. post $"./store/sock//pipe/($id)"
}

