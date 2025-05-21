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

r#'{
  run: {|| websocat "wss://gateway.discord.gg/?v=10&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t | lines },
  duplex: true
}'# | .append discord.ws.spawn

# append the access token to use to the stream
"<token>" | .append discord.ws.token

# add the heartbeat handler to authenticate and maintain an active connection
open examples/discord-bot/handler-heartbeat.nu | .append "discord.heartbeat.register"

# add the discord.nu module for working with discord's REST API
# https://github.com/cablehead/discord.nu
http get https://raw.githubusercontent.com/cablehead/discord.nu/main/discord.nu | .append discord.nu

# we can now register additional handlers to add functionality to the bot
# for example, to enable a `./roll <n>d<m>` command
open examples/discord-bot/handler-roller.nu | .append "discord.roller.register"
```

#### Slash commands

We should be able to make this nicer?

```nushell
# create the command
# see discord.nu
discord app command create 1227338584814649364 dice "make a dice roll" --options [
       (discord app command option int n "number of dice to roll" --required)
       (discord app command option int d "die type / number of sides" --required)
       (discord app command option int modifier "modifier")
   ]

# enable the command handler
open examples/discord-bot/handler-slash-dice.nu | .append "discord.slash-dice.register"
```

### run through

This is a presentation I gave at the [Creative Code Toronto](https://www.meetup.com/creative-code-toronto/) [Sep '24 meetup](https://www.meetup.com/creative-code-toronto/events/303276625/?eventOrigin=group_events_list) :: [slides](https://cablehead.github.io/creative-codie/) :: [video](https://www.youtube.com/watch?v=Y2rsm5ohDrg&list=PL_YfqG2SCOAK52A4VQ7r7m9laijKSbmUB&index=2)

<img src="https://github.com/user-attachments/assets/26bc887f-f3bc-456f-ab16-8913ae414a73" width="600px" />

### deploy on [SidePro](https://sidepro.cloud)

https://github.com/user-attachments/assets/3970a907-899b-4b6c-b7c2-79cab0024d8d
