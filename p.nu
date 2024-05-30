#!/usr/bin/env -S nu --stdin

use xs.nu

alias and-then = if ($in | is-not-empty)

def unpack [chain: closure] {
    get types."public.utf8-plain-text" | decode base64 | do $chain {content_type: "text"}
}

def get-last-id [] {
    xs cat ./store | where topic == "/stream/cross/content" |
        and-then { last | get id }
}

def foo [] {
xs cat ./store --last-id (get-list-id)  | 
    where topic == "/stream/cross/pasteboard" | 
    each {|x|
    }
}

export def main [] {
    let event = $in
    print (pwd)
    print $event.hash
    xs cas ./store $event.hash | from json | unpack  {|meta|
        xs append ./store "/stream/cross/content" --meta (
            $meta | insert link_id $event.id | to json -r)
    }
}
