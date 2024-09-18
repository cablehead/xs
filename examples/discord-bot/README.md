## starter for a discord bot

```
           ┌────────────────────────────────────────┐
           │            Discord Gateway             │
           └────────────▲────────────┳──────────────┘
                        ┃            ┃
                        s            r
                        e            e    op: 10 Hello
    op: 02 Identify     n            c    op: 00 Ready
    op: 01 Heartbeat    d            v    op: 11 Heartbeat ACK
                        ┃            ┃
               ┌────────┻────────────▼────────┐
     ━━ stdin ━▶ $ websocat wss://gatewa...   ┣━ stdout ━▶
               └──────────────────────────────┘
```

Required to run:

- https://github.com/vi/websocat
- [scru128-cli](https://github.com/cablehead/scru128-cli)- needed for `scru128-since`

```
git clone https://github.com/cablehead/xs.git
cargo r -- ./store
```

In another session:

```nushell
use xs.nu *

"websocat "wss://gateway.discord.gg/?v=8&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t | lines" |
    .append discord.ws.spawn --meta {duplex: true}

open examples/discord-bot/handler-heartbeat.nu |
    .append "discord.heartbeat.register" --meta (open examples/discord-bot/handler-heartbeat.nuon)

# to enable a `./roll <n>d<m>` command
open examples/discord-bot/handler-roller.nu | .append "discord.roller.register"
```
