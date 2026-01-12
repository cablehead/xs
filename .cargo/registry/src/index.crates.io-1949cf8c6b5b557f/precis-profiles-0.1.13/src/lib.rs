//! PRECIS Framework: Preparation, Enforcement, and Comparison of
//! Internationalized Strings in Application Protocols as described in
//! [`rfc8264`](https://datatracker.ietf.org/doc/html/rfc8264)
//!
//! This crate implements the next PRECIS profiles:
//! * [`rfc8265`](https://datatracker.ietf.org/doc/html/rfc8265).
//!   Preparation, Enforcement, and Comparison of Internationalized Strings
//!   Representing `Usernames` and `Passwords`.
//! * [`rfc8266`](https://datatracker.ietf.org/doc/html/rfc8266).
//!   Preparation, Enforcement, and Comparison of Internationalized Strings
//!   Representing Nicknames
//!
//! ```rust
//! # use precis_core::profile::PrecisFastInvocation;
//! # use precis_profiles::Nickname;
//! # use std::borrow::Cow;
//! assert_eq!(Nickname::prepare("Guybrush Threepwood"),
//!   Ok(Cow::from("Guybrush Threepwood")));
//! assert_eq!(Nickname::enforce("   Guybrush     Threepwood  "),
//!   Ok(Cow::from("Guybrush Threepwood")));
//! assert_eq!(Nickname::compare("Guybrush   Threepwood  ",
//!   "guybrush threepwood"), Ok(true));
//! ```

#![deny(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/unicode_version.rs"));

mod bidi;
mod common;
mod nicknames;
mod passwords;
mod usernames;

pub use crate::nicknames::Nickname;
pub use crate::passwords::OpaqueString;
pub use crate::usernames::UsernameCaseMapped;
pub use crate::usernames::UsernameCasePreserved;
pub use precis_core;
