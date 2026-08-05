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

  # test .last (bare) returns a record
  .last | describe | str replace --regex '<.*' '' | assert-eq "record" ".last returns record"

  # test .last <topic> returns a record
  .last test-topic | .cas | assert-eq "hello world" ".last content"

  # add more frames for multi-result tests
  "second" | .append test-topic
  "third" | .append test-topic

  # test .last <n> returns a table
  .last 3 | describe | str replace --regex '<.*' '' | assert-eq "table" ".last <n> returns table"

  # test .last <topic> <n> returns a table
  let results = .last test-topic 3
  $results | describe | str replace --regex '<.*' '' | assert-eq "table" ".last <topic> <n> returns table"
  $results | length | assert-eq 3 ".last <topic> <n> count"

  # test .id roundtrip
  let new_id = (.id)
  .id unpack $new_id | .id pack | assert-eq $new_id ".id roundtrip"

  # pinned ids that lost low-order timestamp bits before the round fix.
  # the unpack seconds -> datetime -> pack path truncated ms instead of
  # rounding, so ids whose f64 ms landed just under an integer dropped 1ms.
  let pinned = [
    03gmmrhqljv8bnulus79wipck
    03gmmrhqzjzlbz75e1r8fyvmt
    03gmmrhsh0x17va0knqvbhly6
  ]
  for id in $pinned {
    .id unpack $id | .id pack | assert-eq $id $".id roundtrip pinned ($id)"
  }

  # deterministic loop: many fresh ids must roundtrip exactly.
  # before the fix this failed on roughly 3% of ids.
  let failures = (1..2000 | each {|_|
    let id = (.id)
    let back = (.id unpack $id | .id pack)
    if $back != $id { {id: $id, back: $back} } else { null }
  } | compact)
  $failures | length | assert-eq 0 $".id roundtrip loop failures: ($failures)"

  # test metadata
  "with meta" | .append meta-topic --meta {key: "value"}
  .last meta-topic | get meta.key | assert-eq "value" "metadata"

  # test multi-topic filter: comma string and Nushell list are equivalent
  "a" | .append filt.one
  "b" | .append filt.two
  "c" | .append filt.three
  .cat -T "filt.one,filt.two" | get topic | sort
    | assert-eq ["filt.one", "filt.two"] ".cat -T comma string"
  .cat -T [filt.one filt.two] | get topic | sort
    | assert-eq ["filt.one", "filt.two"] ".cat -T list"

  # test .remove
  .last meta-topic | get id | each {|id| .remove $id }

  print "all xs.nu tests passed"
}
