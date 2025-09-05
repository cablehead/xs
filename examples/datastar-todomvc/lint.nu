#!/usr/bin/env nu

git ls-files README.md ./www ./templates | lines | deno fmt ...$in ; topiary format ./serve.nu
