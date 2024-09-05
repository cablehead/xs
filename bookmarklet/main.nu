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

            let bookmarks = (
                .head "bookmarks" | if ($in | is-not-empty)  {
                    $in | .cas | from json
                } else { {} })

            $bookmarks | upsert $message.d.message_id true |
                to json -r | .append "bookmarks" --meta {id: $frame.id}
            return
        }

        "MESSAGE_REACTION_REMOVE" => {
            if $message.d.emoji.name != "🔖" { return }

            let bookmarks = (
                .head "bookmarks" | if ($in | is-not-empty)  {
                    $in | .cas | from json
                } else { {} })

            $bookmarks | reject -i $message.d.message_id |
                to json -r | .append "bookmarks" --meta {id: $frame.id}
            return
        }
    }

    return $message
}
