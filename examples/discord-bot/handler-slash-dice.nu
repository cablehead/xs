## Really need a way to store modules on the stream, which can be imported by handlers
## https://github.com/cablehead/discord.nu/blob/main/discord/mod.nu
## -->

# Create Interaction Response
# https://discord.com/developers/docs/interactions/receiving-and-responding#create-interaction-response
const API_BASE = "https://discord.com/api/v10"
export def "interaction response" [
    interaction_id: string
    interaction_token: string
    content: string
    --type: int = 4
] {
    let url = $"($API_BASE)/interactions/($interaction_id)/($interaction_token)/callback"
    http post --content-type application/json $url {
        type: $type
        data: {
            content: $content
        }
    }
}

def run-dice [options: record] {
   let dice = (random dice --dice $options.n --sides $options.d)
   mut content = ($dice | each { $"($in) <:nondescript_die:1227997035945267232>" } | str join " + ")


   if $options.modifier != 0 {
       $content += $" + ($options.modifier)"
   }

   $content += $" == ($options.modifier + ($dice | math sum))"
   $content
}

{|frame|
    if $frame.topic != "discord.ws.recv" { return }

    let message = $frame | .cas $in.hash | from json
    if $message.op != 0 { return }
    if $message.t != "INTERACTION_CREATE" { return }

    let command = $message.d.data
    if $command.name != "dice" { return }

    let options = (
        $command.options |
        each {|x| {$x.name: $x.value}} |
        reduce {|it, acc| $it | merge $acc} |
        default 0 modifier
    )
    let content = run-dice $options

    $message.d | interaction response $in.id $in.token $content
}
