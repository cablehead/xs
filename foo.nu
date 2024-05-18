#!/usr/bin/env -S nu --stdin

curl -sN --unix-socket ./store/sock 'localhost/' | lines | each { from json }
