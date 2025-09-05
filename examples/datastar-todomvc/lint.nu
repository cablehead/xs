#!/usr/bin/env nu

git ls-files README.md ./www ./templates | lines | deno fmt ...$in
git ls-files | lines | where { str ends-with ".nu" } | topiary format ...$in
