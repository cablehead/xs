
def "scru128-since" [$id1, $id2] {
    let t1 = ($id1 | scru128 parse | into int)
    let t2 = ($id2 | scru128 parse | into int)
    return ($t1 - $t2)
}

let the_init = {||
    {
        last_id: null,
        s: null, # sequence number
        heartbeat_interval: 0, # 0 means we are offline
        last_sent: null,
        last_ack: null,

        authing: null,
        session_id: null,
        resume_gateway_url: null,
    }
}


let the_thing = {|state|
    mut state = $state
    let frame = $in
    if $frame.topic != "discord.recv" {
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
        },
    }

    $state
}
