[![Docs](https://docs.rs/precis-profiles/badge.svg)](https://docs.rs/precis-profiles)
[![Crates.io](https://img.shields.io/crates/v/precis-profiles)](https://crates.io/crates/precis-profiles)

# precis-profiles

PRECIS Framework: Preparation, Enforcement, and Comparison of
Internationalized Strings in Application Protocols as described in
[rfc8264](https://datatracker.ietf.org/doc/html/rfc8264)

This crate implements the next PRECIS profiles:
 * [rfc8265](https://datatracker.ietf.org/doc/html/rfc8265).
   Preparation, Enforcement, and Comparison of Internationalized Strings
   Representing Usernames and Passwords.
 * [rfc8266](https://datatracker.ietf.org/doc/html/rfc8266).
   Preparation, Enforcement, and Comparison of Internationalized Strings
   Representing Nicknames

## Examples
```rust
assert_eq!(Nickname::prepare("Guybrush Threepwood"),
  Ok(Cow::from("Guybrush Threepwood")));
assert_eq!(Nickname::enforce("   Guybrush     Threepwood  "),
  Ok(Cow::from("Guybrush Threepwood")));
assert_eq!(Nickname::compare("Guybrush   Threepwood  ",
  "guybrush threepwood"), Ok(true));
```

# Contributing

Patches and feedback are welcome.

# Donations

If you find this project helpful, you may consider making a donation:

<img src="https://www.bitcoinqrcodemaker.com/api/?style=bitcoin&amp;address=bc1qx258lwvgzlg5zt2xsns2nr75dhvxuzk3wkqmnh" height="150" width="150" alt="Bitcoin QR Code">
<img src="https://www.bitcoinqrcodemaker.com/api/?style=ethereum&amp;address=0xefa6404e5A50774117fd6204cbD33cf4454c67Fb" height="150" width="150" alt="Ethereum QR Code">

# License

This project is licensed under either of
* [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
* [MIT license](https://opensource.org/licenses/MIT)

[![say thanks](https://img.shields.io/badge/Say%20Thanks-👍-1EAEDB.svg)](https://github.com/sancane/precis/stargazers)
