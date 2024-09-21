# op.nu
# we need a mechanism to be able to reuse snippets of code
const opcode = {
    dispatch: 0,
    heartbeat: 1,
    identify: 2,
    presence_update: 3,
    voice_update: 4,
    resume: 6,
    reconnect: 7,
    invalid_session: 9,
    hello: 10,
    heartbeat_ack: 11,
}

def "op heartbeat" [seq?: int] {
    {
        "op": $opcode.heartbeat,
        "d": $seq,
    }
}

def "op identify" [token: string, intents: int] {
    {
        "op": $opcode.identify,
        "d": {
            token: $token,
            intents: $intents,
            properties: {
                os: (sys host | get name),
                browser: "discord.nu",
                device: "discord.nu",
            },
        },
    }
}

def "op resume" [token: string, session_id: string, seq: int] {
    {
        "op": $opcode.resume,
        "d": {
            token: $token,
            session_id: $session_id,
            seq: $seq,
        },
    }
}
### end op.nu

def "scru128-since" [$id1, $id2] {
    let t1 = ($id1 | scru128 parse | into int)
    let t2 = ($id2 | scru128 parse | into int)
    return ($t1 - $t2)
}

def .send [] {
    to json -r | $"($in)\n" | .append "discord.ws.send" --ttl ephemeral
}

{|state|
    mut state = $state
    let frame = $in

    # https://discord.com/developers/docs/topics/gateway#list-of-intents
    # GUILDS, GUILD_MEMBERS, GUILD_MESSAGES, GUILD_MESSAGE_REACTIONS, MESSAGE_CONTENT
    let IDENTIFY_INTENTS = 34307

    if $frame.topic == "xs.pulse" {
        # we're not online
        if $state.heartbeat_interval == 0 {
            return
        }

        # online, but not authed, attempt to auth
        if (($state.heartbeat_interval != 0) and ($state.authing | is-empty)) {
            op identify $env.BOT_TOKEN $IDENTIFY_INTENTS | .send
            return
        }

        let since = (scru128-since $frame.id $state.last_sent)
        let interval =  (($state.heartbeat_interval / 1000) * 0.9)
        if ($since > $interval) {
            op heartbeat | .send
        }
        return
    }

    if $frame.topic not-in ["discord.ws.recv" "discord.ws.send"] {
        return
    }

    let message = $frame | .cas $in.hash | from json

    match $message {
        # hello
        {op: 10} => {
            $state.heartbeat_interval = $message.d.heartbeat_interval
            $state.last_ack = $frame.id
            $state.last_sent = $frame.id
            $state.authing = null
        }

        # heartbeat
        {op: 1} => {
            $state.last_ack = null
            $state.last_sent = $frame.id
        }

        # heartbeat_ack
        {op: 11} => {
            $state.last_ack = $frame.id
            .rm $frame.id
        }

        # identify
        {op: 2} => {
            $state.authing = "identify"
        }

        # resume
        {op: 6} => {
            $state.authing = "resume"
        }

        # invalid_session
        {op: 9} => {
            # The inner d key is a boolean that indicates whether the session may be resumable.
            # if we get an invalid session while trying to resume, also clear
            # out the session
            if not $message.d or $state.authing == "resume" {
                $state.resume_gateway_url = null
                $state.session_id = null
            }
            $state.authing = null
        }

        # dispatch:: READY
        {op: 0, t: "READY"} => {
            $state.session_id = $message.d.session_id
            $state.resume_gateway_url = $message.d.resume_gateway_url
            $state.authing = "authed"
        }

        # dispatch:: RESUMED
        {op: 0, t: "RESUMED"} => {
            $state.authing = "authed"
        }

        # dispatch:: GUILD_CREATE
        {op: 0, t: "GUILD_CREATE"} => {
            # ignore
        }
    }

    { state: $state }
}
