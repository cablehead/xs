use op.nu *

def "scru128-since" [$id1, $id2] {
    let t1 = ($id1 | scru128 parse | into int)
    let t2 = ($id2 | scru128 parse | into int)
    return ($t1 - $t2)
}

let the_init = {||
    {
        s: null,               # sequence number
        heartbeat_interval: 0, # 0 means we are offline
        last_sent: null,
        last_ack: null,

        authing: null,
        session_id: null,
        resume_gateway_url: null,
    }
}

def .send [] {
    to json -r | $"($in)\n" | .append "discord.send"
}

let the_thing = {|state|
    mut state = $state
    let frame = $in

    if $frame.topic == "xs.pulse" {
        # if we're online, but not authed, attempt to auth
        if (($state.heartbeat_interval != 0) and ($state.authing | is-empty)) {
            print "sending identify!"
            op identify $env.BOT_TOKEN 33281 | .send
            return
        }

        let since = (scru128-since $frame.id $state.last_sent)
        let interval =  (($state.heartbeat_interval / 1000) * 0.9)
        if ($since > $interval) {
            op heartbeat | .send
        }
        return
    }

    if $frame.topic not-in ["discord.recv" "discord.send"] {
        return
    }

    let message = $frame | .cas | from json

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

        _ => {
            $frame | to json -r | $"($in)\n" | .append "discord.todo"
        }
    }

    $state
}