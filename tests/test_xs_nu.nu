#!/usr/bin/env nu

# Integration test for xs.nu overlay
# Run with: nu tests/test_xs_nu.nu

def assert-eq [expected: any, msg: string = "assertion failed"] {
  let actual = $in
  if $actual != $expected {
    error make {msg: $"($msg): expected '($expected)', got '($actual)'"}
  }
}

use ../xs.nu *

.tmp-spawn {
  # test .append, .cat, .cas
  "hello world" | .append test-topic
  .cat --last 1 | first | .cas | assert-eq "hello world" ".cas content"

  # test .get
  let id = (.cat --last 1 | first | get id)
  .get $id | get topic | assert-eq "test-topic" ".get topic"

  # test .last
  .last test-topic | .cas | assert-eq "hello world" ".last content"

  # test .id roundtrip
  let new_id = (.id)
  .id unpack $new_id | .id pack | assert-eq $new_id ".id roundtrip"

  # test metadata
  "with meta" | .append meta-topic --meta {key: "value"}
  .last meta-topic | get meta.key | assert-eq "value" "metadata"

  # test .remove
  .last meta-topic | get id | each {|id| .remove $id }

  print "all xs.nu tests passed"
}
