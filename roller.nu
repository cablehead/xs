def & [action: closure] {
    if ($in | is-not-empty) {
        $in | do $action
    }
}

def map-values [closure: closure] {
    transpose  | each { update column1 { do $closure } } | transpose --header-row -d
}

def send-message [channel_id: string] {
    let data = $in
    let headers = {
        Authorization: $"Bot ($env.BOT_TOKEN)",
    }
    let url = $"https://discord.com/api/v9/channels/($channel_id)/messages"
    http post --content-type application/json  --headers $headers $url $data
}

def parse-roller [] {
    parse --regex '\./roll (?P<dice>\d+)d(?P<sides>\d+)(?:\+(?P<modifier>\d+))?' | & {
        update modifier { if $in == "" { "0" } else { $in } } | map-values { into int }
    }
}

def run-roll [] {
   let roll = $in

   let dice = (random dice --dice $roll.dice --sides $roll.sides)

   mut content = ($dice | each { $"($in) <:nondescript_die:1227997035945267232>" } | str join " + ")

   if $roll.modifier != 0 {
       $content += $" + ($roll.modifier)"
   }

   $content += $" == ($roll.modifier + ($dice | math sum))"
   $content
}

{||
    let frame = $in
    if $frame.topic != "discord.recv" { return }
    let message = $frame | .cas | from json
    if $message.op != 0 { return }
    if $message.t != "MESSAGE_CREATE" { return }
    $message.d.content | parse-roller | & {
        {
            content: ($in | run-roll),
            message_reference: { message_id: $message.d.id },
        } | send-message $message.d.channel_id
    }
}
