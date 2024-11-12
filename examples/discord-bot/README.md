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
 ━━ stdin ━▶  $ websocat wss://gatewa...  ┣━ stdout ━▶
          ▲└──────────────────────────────┘  │
          │                                  │
discord.ws.send                      discord.ws.recv
          │                                  │
         ┌┴──────────────────────────────────▼─┐
         │         $ xs serve ./store          │
         └─────────────────────────────────────┘
```

Required to run:

- https://github.com/vi/websocat
- [scru128-cli](https://github.com/cablehead/scru128-cli)- needed for `scru128-since`

```
% xs serve ./store
```

In another session:

```nushell
use xs.nu *

"websocat "wss://gateway.discord.gg/?v=8&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t | lines" |
    .append discord.ws.spawn --meta {duplex: true}

# append the access token to use to the stream
"<token>" | .append discord.ws.token

open examples/discord-bot/handler-heartbeat.nu |
    .append "discord.heartbeat.register" --meta (open examples/discord-bot/handler-heartbeat.nuon)

# to enable a `./roll <n>d<m>` command
open examples/discord-bot/handler-roller.nu | .append "discord.roller.register"
```

### run through

This is a presentation I gave at the [Creative Code Toronto](https://www.meetup.com/creative-code-toronto/) [Sep '24 meetup](https://www.meetup.com/creative-code-toronto/events/303276625/?eventOrigin=group_events_list)

<img src="https://github.com/user-attachments/assets/26bc887f-f3bc-456f-ab16-8913ae414a73" width="600px" />

- [slides](https://cablehead.github.io/creative-codie/)
- [video](https://www.youtube.com/watch?v=Y2rsm5ohDrg&list=PL_YfqG2SCOAK52A4VQ7r7m9laijKSbmUB&index=2)
