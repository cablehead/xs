## starter for a discord bot

Requires:

- https://www.nushell.sh
- https://github.com/vi/websocat

```
git clone https://github.com/cablehead/xs.git
cargo r -- ./store
```

In another session:

```nushell
use xs2.nu *

"websocat "wss://gateway.discord.gg/?v=8&encoding=json" --ping-interval 5 --ping-timeout 10 -E -t" |
    .append xs.generator.spawn --meta {topic: "discord" duplex: true}
```



