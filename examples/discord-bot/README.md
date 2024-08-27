## starter for a discord bot

Requires:

- https://www.nushell.sh
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
    .append xs.generator.spawn --meta {topic: "discord" duplex: true}
```



