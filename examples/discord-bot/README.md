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
    .append discord.ws.spawn --meta {duplex: true}

open examples/discord-bot/handler-heartbeat.nu |
    .append "discord.heartbeat.register" --meta (open examples/discord-bot/handler-heartbeat.nuon)

# to enable a `./roll <n>d<m>` command
open examples/discord-bot/handler-roller.nu | .append "discord.roller.register"
```
