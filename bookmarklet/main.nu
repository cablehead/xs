{|state frame|
    if $frame.topic != "discord.ws.recv" { return }

    let message = ($frame | .cas $in.hash | from json)
    if $message.op != 0 { return }

    match $message.t {
        "READY" => return
    }

    match $message.t {
        "MESSAGE_CREATE" | "MESSAGE_DELETE" | "MESSAGE_UPDATE" => {
            .append $"message.($message.d.id)" --meta {id: $frame.id}
            return
        }

        "MESSAGE_REACTION_ADD" => {
            if $message.d.emoji.name != "🔖" { return }
            return $message
        }
    }

    return $message

    mut state = $state

    match $message.t {
        "MESSAGE_CREATE" => (
            $state.messages = ($state.messages | insert $message.d.id $message.d.content))

        "MESSAGE_UPDATE" => (
            $state.messages = ($state.messages | upsert $message.d.id $message.d.content))

        "MESSAGE_DELETE" => (
            $state.marked = ($state.marked | reject -i $message.d.id))

        "MESSAGE_REACTION_ADD" => (
            if $message.d.emoji.name == "🔖" {
                $state.marked = ($state.marked | insert $message.d.message_id true) }
        )

        "MESSAGE_REACTION_REMOVE" => (
            if $message.d.emoji.name == "🔖" {
                $state.marked = ($state.marked | reject -i $message.d.message_id) }
        )
    }

    let messages = $state.messages

  {state: $state}
}
