use xs2.nu *

source ./examples/discord-bot/beat.nu

let start = (
    .cat | where topic == "discord.start"
    | if ($in | is-not-empty) { last } else { print "no start"; exit })

let state = do $the_init

.cat -fp 1000 -l $start.id | stateful filter $state {|state frame|
    clear
    print $state $frame
    let update = $frame | do $the_thing $state
    if ($update | is-empty) { return {} }
    {state: $update}
}
