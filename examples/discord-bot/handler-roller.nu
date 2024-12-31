def & [action: closure] {
  if ($in | is-not-empty) {
    $in | do $action
  }
}

def map-values [closure: closure] {
  transpose | each {update column1 {do $closure}} | transpose --header-row -d
}

def parse-roller [] {
  parse --regex '\./roll (?P<dice>\d+)d(?P<sides>\d+)(?:\+(?P<modifier>\d+))?' | & {
    update modifier {if $in == "" {"0"} else {$in}} | map-values {into int}
  }
}

def run-roll [] {
  let roll = $in

  let dice = (random dice --dice $roll.dice --sides $roll.sides)

  mut content = ($dice | each {$"($in) <:nondescript_die:1227997035945267232>"} | str join " + ")

  if $roll.modifier != 0 {
    $content += $" + ($roll.modifier)"
  }

  $content += $" == ($roll.modifier + ($dice | math sum))"
  $content
}

$env.BOT_TOKEN = .head discord.ws.token | .cas $in.hash

{|frame|
  if $frame.topic != "discord.ws.recv" { return }

  # TODO: .cas should also be able to take a record, to match xs2.nu's usage
  let message = $frame | .cas $in.hash | from json

  if $message.op != 0 { return }
  if $message.t != "MESSAGE_CREATE" { return }

  $message.d.content | parse-roller | & {
    {
      content: ($in | run-roll)
      message_reference: { message_id: $message.d.id }
    } | discord channel message create $message.d.channel_id
  }
}
