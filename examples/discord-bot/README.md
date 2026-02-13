## run through

This is a presentation I gave at the
[Creative Code Toronto](https://www.meetup.com/creative-code-toronto/)
[Sep '24 meetup](https://www.meetup.com/creative-code-toronto/events/303276625/?eventOrigin=group_events_list)
:: [slides](https://cablehead.github.io/creative-codie/) ::
[video](https://www.youtube.com/watch?v=Y2rsm5ohDrg&list=PL_YfqG2SCOAK52A4VQ7r7m9laijKSbmUB&index=2)

<img src="https://github.com/user-attachments/assets/26bc887f-f3bc-456f-ab16-8913ae414a73" width="600px" />

## deploy on [SidePro](https://sidepro.cloud)

https://github.com/user-attachments/assets/3970a907-899b-4b6c-b7c2-79cab0024d8d

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

Clone the repository:

```
git clone https://github.com/cablehead/xs.git
cd xs
```

Start the xs server:

```
% xs serve ./store
```

In another session:

Load xs.nu and set store path:

```nushell
use xs.nu *
$env.XS_ADDR = "./store" | path expand
```

Spawn Discord websocket connection:

```nushell
r#'{
  run: {|| websocat "wss://gateway.discord.gg/?v=10&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t | lines },
  duplex: true
}'# | .append discord.ws.spawn
```

Add your Discord bot token:

```nushell
"<token>" | .append discord.ws.token
```

Register heartbeat actor for authentication:

```nushell
open examples/discord-bot/actor-heartbeat.nu | .append "discord.heartbeat.register"
```

At this point, all messages sent to the Discord server will be available on the
event stream. You can build from there, creating actors that take action for
specific messages. For example, we could register an actor that looks for
messages in the form `./roll 1d4` and responds with a dice roll.

Load Discord REST API module:

```nushell
http get https://raw.githubusercontent.com/cablehead/discord.nu/main/discord.nu | .append discord.nu
```

Register dice roll actor:

```nushell
open examples/discord-bot/actor-roller.nu | .append "discord.roller.register"
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

# enable the command actor
open examples/discord-bot/actor-slash-dice.nu | .append "discord.slash-dice.register"
```
