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
    let t1 = ($id1 | .id unpack | get timestamp)
    let t2 = ($id2 | .id unpack | get timestamp)
    return ($t1 - $t2)
}

def .send [] {
    to json -r | $"($in)\n" | .append "discord.ws.send" --ttl last:5
}

$env.BOT_TOKEN = .last discord.ws.token | .cas $in.hash

{
  start: (.last discord.ws.running | get -i id | default "first")
  pulse: 1000

  initial: {
    s: null,
    heartbeat_interval: 0,
    last_sent: null,
    last_ack: null,
    authing: null,
    session_id: null,
    resume_gateway_url: null
  }

  run: {|frame, state|
    # https://discord.com/developers/docs/topics/gateway#list-of-intents
    # GUILDS, GUILD_MEMBERS, GUILD_MESSAGES, GUILD_MESSAGE_REACTIONS, MESSAGE_CONTENT
    let IDENTIFY_INTENTS = 34307

    if $frame.topic == "xs.pulse" {
        # we're not online
        if $state.heartbeat_interval == 0 {
            return {next: $state}
        }

        # online, but not authed, attempt to auth
        if (($state.heartbeat_interval != 0) and ($state.authing | is-empty)) {
            op identify $env.BOT_TOKEN $IDENTIFY_INTENTS | .send
            return {next: ($state | merge {authing: "identify"})}
        }

        let since = (scru128-since $frame.id $state.last_sent)
        let interval = (($state.heartbeat_interval * 0.9) * 1ms)
        if ($since > $interval) {
            op heartbeat | .send
            return {next: ($state | merge {last_ack: null, last_sent: $frame.id})}
        }
        return {next: $state}
    }

    if $frame.topic != "discord.ws.recv" {
        return {next: $state}
    }

    let message = $frame.meta

    match $message {
        # hello
        {op: 10} => {
            {next: ($state | merge {
                heartbeat_interval: $message.d.heartbeat_interval,
                last_ack: $frame.id,
                last_sent: $frame.id,
                authing: null
            })}
        }

        # heartbeat_ack
        {op: 11} => {
            .rm $frame.id
            {next: ($state | merge {last_ack: $frame.id})}
        }

        # resume
        {op: 6} => {
            {next: ($state | merge {authing: "resume"})}
        }

        # invalid_session
        {op: 9} => {
            # The inner d key is a boolean that indicates whether the session may be resumable.
            # if we get an invalid session while trying to resume, also clear
            # out the session
            if not $message.d or $state.authing == "resume" {
                {next: ($state | merge {
                    resume_gateway_url: null,
                    session_id: null,
                    authing: null
                })}
            } else {
                {next: ($state | merge {authing: null})}
            }
        }

        # dispatch:: READY
        {op: 0, t: "READY"} => {
            {next: ($state | merge {
                session_id: $message.d.session_id,
                resume_gateway_url: $message.d.resume_gateway_url,
                authing: "authed"
            })}
        }

        # dispatch:: RESUMED
        {op: 0, t: "RESUMED"} => {
            {next: ($state | merge {authing: "authed"})}
        }

        # dispatch:: GUILD_CREATE
        {op: 0, t: "GUILD_CREATE"} => {
            # ignore
            {next: $state}
        }

        _ => {
            {next: $state}
        }
    }
  }
}
