[![Crates.io](https://img.shields.io/crates/v/stun-rs)](https://crates.io/crates/stun-rs)
[![Docs](https://img.shields.io/docsrs/stun-rs/latest)](https://docs.rs/stun-rs)
[![codecov](https://codecov.io/gh/sancane/rustun/branch/main/graph/badge.svg?token=19Juem5PrN)](https://codecov.io/gh/sancane/rustun)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/sancane/rustun)](https://rust-reportcard.xuri.me/report/github.com/sancane/rustun)

# Crate stun-rs

This crate provides a simple but high effective framework to manage STUN protocol messages. The implementation is based on:
* [`RFC5769`](https://datatracker.ietf.org/doc/html/rfc5769). Test Vectors for Session Traversal Utilities for NAT (STUN).
* [`RFC5780`](https://datatracker.ietf.org/doc/html/rfc5780). NAT Behavior Discovery Using Session Traversal Utilities for NAT (STUN).
* [`RFC8016`](https://datatracker.ietf.org/doc/html/rfc8016). Mobility with Traversal Using Relays around NAT (TURN).
* [`RFC8445`](https://datatracker.ietf.org/doc/html/rfc8445). Interactive Connectivity Establishment (ICE).
* [`RFC8489`](https://datatracker.ietf.org/doc/html/rfc8489). Session Traversal Utilities for NAT (STUN).
* [`RFC8656`](https://datatracker.ietf.org/doc/html/rfc8656). Traversal Using Relays around NAT (TURN).


## Usage
Example that creates and encodes a STUN Binding request
```rust
 // Create attributes
 let username = UserName::new("\u{30DE}\u{30C8}\u{30EA}\u{30C3}\u{30AF}\u{30B9}")?;
 let nonce = Nonce::new("f//499k954d6OL34oL9FSTvy64sA")?;
 let realm = Realm::new("example.org")?;
 let password = "TheMatrIX";
 let algorithm = Algorithm::from(AlgorithmId::MD5);
 let key = HMACKey::new_long_term(&username, &realm, password, algorithm)?;
 let integrity = MessageIntegrity::new(key);

 // Create the message
 let msg = StunMessageBuilder::new(
   BINDING,
   MessageClass::Request,
 )
 .with_attribute(username)
 .with_attribute(nonce)
 .with_attribute(realm)
 .with_attribute(integrity)
 .build();

 // Create an encoder to encode the message into a buffer
 let encoder = MessageEncoderBuilder::default().build();
 let mut buffer: [u8; 150] = [0x00; 150];
 let size = encoder.encode(&mut buffer, &msg)?;
 assert_eq!(size, 116);
```

Example that decodes a STUN Binding response and fetches some attributes.
```rust
 // This response uses the following parameter:
 // Password: `VOkJxbRl1RmTxUk/WvJxBt` (without quotes)
 // Software name: "test vector" (without quotes)
 // Mapped address: 192.0.2.1 port 32853
 let sample_ipv4_response = [
     0x01, 0x01, 0x00, 0x3c, // Response type and message length
     0x21, 0x12, 0xa4, 0x42, // Magic cookie
     0xb7, 0xe7, 0xa7, 0x01, // }
     0xbc, 0x34, 0xd6, 0x86, // }  Transaction ID
     0xfa, 0x87, 0xdf, 0xae, // }
     0x80, 0x22, 0x00, 0x0b, // SOFTWARE attribute header
     0x74, 0x65, 0x73, 0x74, // }
     0x20, 0x76, 0x65, 0x63, // }  UTF-8 server name
     0x74, 0x6f, 0x72, 0x20, // }
     0x00, 0x20, 0x00, 0x08, // XOR-MAPPED-ADDRESS attribute header
     0x00, 0x01, 0xa1, 0x47, // Address family (IPv4) and xor'd mapped port number
     0xe1, 0x12, 0xa6, 0x43, // Xor'd mapped IPv4 address
     0x00, 0x08, 0x00, 0x14, // MESSAGE-INTEGRITY header
     0x2b, 0x91, 0xf5, 0x99, // }
     0xfd, 0x9e, 0x90, 0xc3, // }
     0x8c, 0x74, 0x89, 0xf9, // } HMAC-SHA1 fingerprint
     0x2a, 0xf9, 0xba, 0x53, // }
     0xf0, 0x6b, 0xe7, 0xd7, // }
     0x80, 0x28, 0x00, 0x04, // FINGERPRINT attribute header
     0xc0, 0x7d, 0x4c, 0x96, // Reserved for CRC32 fingerprint
 ];

 // Create a STUN decoder context using the password as a short credential
 // mechanism and force validation of MESSAGE-INTEGRITY and FINGERPRINT
 let ctx = DecoderContextBuilder::default()
   .with_key(
     HMACKey::new_short_term("VOkJxbRl1RmTxUk/WvJxBt")?,
   )
   .with_validation()
   .build();
 let decoder = MessageDecoderBuilder::default().with_context(ctx).build();

 let (msg, size) = decoder.decode(&sample_ipv4_response)?;
 assert_eq!(size, sample_ipv4_response.len());

 // Check message method is a BINDING response
 assert_eq!(msg.method(), BINDING);
 assert_eq!(msg.class(), MessageClass::SuccessResponse);

 let software = msg.get::<Software>()
   .ok_or("Software attribute not found")?
   .as_software()?;
 assert_eq!(software, "test vector");

 let xor_addr = msg.get::<XorMappedAddress>()
   .ok_or("XorMappedAddress attribute not found")?
   .as_xor_mapped_address()?;
 let socket = xor_addr.socket_address();
 assert_eq!(socket.ip(), IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
 assert_eq!(socket.port(), 32853);
 assert!(socket.is_ipv4());
```

# Common features

This crate defines next feature flags that can be enabled:
* **discovery**: Extends support for parsing attributes defined in [`RFC5780`](https://datatracker.ietf.org/doc/html/rfc5780). NAT Behavior Discovery Using Session Traversal Utilities for NAT (STUN).
* **mobility**: Extends support for parsing attributes defined in [`RFC8016`](https://datatracker.ietf.org/doc/html/rfc8016). Mobility with Traversal Using Relays around NAT (TURN).
* **turn**: Extends support for parsing attributes defined in [`RFC8656`](https://datatracker.ietf.org/doc/html/rfc8656). Traversal Using Relays around NAT (TURN).
* **ice**: Extends support for parsing attributes defined in [`RFC8445`](https://datatracker.ietf.org/doc/html/rfc8445). Interactive Connectivity Establishment (ICE).
* **experiments**: This flag can be set to adjust some behavior of the library, such as default padding. When testing protocols, we can use this flag to force the library to keep the data associated with unknown attributes. By default, unknown attributes store no data to save memory consumption.


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

[![say thanks](https://img.shields.io/badge/Say%20Thanks-👍-1EAEDB.svg)](https://github.com/sancane/rustun/stargazers)
