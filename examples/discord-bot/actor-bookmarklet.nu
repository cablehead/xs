{
  run: {|frame, state|
    if $frame.topic != "discord.ws.recv" { return {next: $state} }

    let message = $frame.meta
    if $message.op != 0 { return {next: $state} }

    match $message.t {
        "READY" | "GUILD_MEMBER_ADD" => { return {next: $state} }

        "MESSAGE_CREATE" | "MESSAGE_DELETE" | "MESSAGE_UPDATE" => {
            .append $"message.($message.d.id)" --meta {id: $frame.id}
            return {next: $state}
        }

        "MESSAGE_REACTION_ADD" => {
            if $message.d.emoji.name != "🔖" { return {next: $state} }

            let bookmarks = (
                .last "bookmarks" | if ($in | is-not-empty)  {
                    $in | .cas | from json
                } else { {} })

            $bookmarks | upsert $message.d.message_id true |
                to json -r | .append "bookmarks" --meta {id: $frame.id}
            return {next: $state}
        }

        "MESSAGE_REACTION_REMOVE" => {
            if $message.d.emoji.name != "🔖" { return {next: $state} }

            let bookmarks = (
                .last "bookmarks" | if ($in | is-not-empty)  {
                    $in | .cas | from json
                } else { {} })

            $bookmarks | reject -i $message.d.message_id |
                to json -r | .append "bookmarks" --meta {id: $frame.id}
            return {next: $state}
        }
    }

    {out: $message, next: $state}
  }
}
