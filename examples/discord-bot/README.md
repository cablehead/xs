## starter for a discord bot

Required to run:

- https://github.com/vi/websocat
- [scru128-cli](https://github.com/cablehead/scru128-cli)- needed for `scru128-since`

```
git clone https://github.com/cablehead/xs.git
cargo r -- ./store
```

In another session:

```nushell
use xs2.nu *

"websocat "wss://gateway.discord.gg/?v=8&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t | lines" |
    .append discord.spawn --meta {duplex: true}

open examples/discord-bot/beat.nu | .append "discord.heartbeat.register" --meta {
    stateful: true
    initial_state: {
       s: null,               # sequence number
       heartbeat_interval: 0, # 0 means we are offline
       last_sent: null,
       last_ack: null,

       authing: null,
       session_id: null,
       resume_gateway_url: null,
   }
   pulse: 1000
}

# to enable a `./roll <n>d<m>` command
open examples/discord-bot/roller.nu | .append "discord.roller.register"
```
