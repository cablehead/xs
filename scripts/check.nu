#!/usr/bin/env nu

let result = (^prek run --all-files | complete)

if $result.stdout != "" {
  print $result.stdout
}

if $result.stderr != "" {
  print $result.stderr
}

if $result.exit_code != 0 {
  exit $result.exit_code
}
