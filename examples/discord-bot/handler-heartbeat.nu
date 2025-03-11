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
                device: "xs",
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
    to json -r | $"($in)\n" | .append "discord.ws.send" --ttl head:5
}

$env.state = {
    s: null,
    heartbeat_interval: 0,
    last_sent: null,
    last_ack: null,
    authing: null,
    session_id: null,
    resume_gateway_url: null
  }

$env.BOT_TOKEN = .head discord.ws.token | .cas $in.hash

{
  resume_from: (.head discord.ws.start | if ($in | is-not-empty) { get id })
  pulse: 1000

  run: {|frame|
    # https://discord.com/developers/docs/topics/gateway#list-of-intents
    # GUILDS, GUILD_MEMBERS, GUILD_MESSAGES, GUILD_MESSAGE_REACTIONS, MESSAGE_CONTENT
    let IDENTIFY_INTENTS = 34307

    if $frame.topic == "xs.pulse" {
        # we're not online
        if $env.state.heartbeat_interval == 0 {
            return
        }

        # online, but not authed, attempt to auth
        if (($env.state.heartbeat_interval != 0) and ($env.state.authing | is-empty)) {
            op identify $env.BOT_TOKEN $IDENTIFY_INTENTS | .send
            $env.state.authing = "identify"
            return
        }

        let since = (scru128-since $frame.id $env.state.last_sent)
        let interval =  (($env.state.heartbeat_interval / 1000) * 0.9)
        if ($since > $interval) {
            op heartbeat | .send
            $env.state.last_ack = null
            $env.state.last_sent = $frame.id
            return
        }
        return
    }

    if $frame.topic != "discord.ws.recv" {
        return
    }

    let message = $frame | .cas $in.hash | from json

    match $message {
        # hello
        {op: 10} => {
            $env.state.heartbeat_interval = $message.d.heartbeat_interval
            $env.state.last_ack = $frame.id
            $env.state.last_sent = $frame.id
            $env.state.authing = null
        }

        # heartbeat_ack
        {op: 11} => {
            $env.state.last_ack = $frame.id
            .rm $frame.id
        }

        # resume
        {op: 6} => {
            $env.state.authing = "resume"
        }

        # invalid_session
        {op: 9} => {
            # The inner d key is a boolean that indicates whether the session may be resumable.
            # if we get an invalid session while trying to resume, also clear
            # out the session
            if not $message.d or $env.state.authing == "resume" {
                $env.state.resume_gateway_url = null
                $env.state.session_id = null
            }
            $env.state.authing = null
        }

        # dispatch:: READY
        {op: 0, t: "READY"} => {
            $env.state.session_id = $message.d.session_id
            $env.state.resume_gateway_url = $message.d.resume_gateway_url
            $env.state.authing = "authed"
        }

        # dispatch:: RESUMED
        {op: 0, t: "RESUMED"} => {
            $env.state.authing = "authed"
        }

        # dispatch:: GUILD_CREATE
        {op: 0, t: "GUILD_CREATE"} => {
            # ignore
        }
    }
  }
}
