#!/usr/bin/env -S nu --stdin

def h. [path: string] {
    curl -sN --unix-socket ./store/sock $"'localhost($path)'" | lines | each { from json }
}

let clip = ( h. / | first )

$clip.hash

h. $"/cas/($clip.hash)"
