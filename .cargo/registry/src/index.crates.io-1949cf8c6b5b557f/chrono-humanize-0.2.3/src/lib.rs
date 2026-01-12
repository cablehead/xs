//! Representation for chrono objects in human languages
//!
//! # Quick Start
//!
//! `HumanTime` objects are created from chrono objects, such as `chrono::DateTime`
//! and `chrono::Duration`
//!
//! # Examples
//!
//! Convert current time taken as `Local::now()` to `HumanTime`
//!
//! ```
//! let dt = chrono::Local::now();
//! let ht = chrono_humanize::HumanTime::from(dt);
//!
//! assert_eq!("now", format!("{}", ht));
//! ```
//!
//!
//! ```
//! let dt = chrono::Local::now() - chrono::Duration::minutes(58);
//! let ht = chrono_humanize::HumanTime::from(dt);
//!
//! assert_eq!("an hour ago", format!("{}", ht));
//! ```
//!
//! For full control over the text representation use `HumanTime::to_text_en()`
//!
//! ```
//! use chrono::Duration;
//! use chrono_humanize::{Accuracy, HumanTime, Tense};
//!
//! # fn main() {
//! let dt = Duration::days(45);
//! let ht = HumanTime::from(dt);
//!
//! assert_eq!("a month", ht.to_text_en(Accuracy::Rough, Tense::Present));
//! assert_eq!("1 month, 2 weeks and 1 day", ht.to_text_en(Accuracy::Precise, Tense::Present));
//! # }
//! ```

#![cfg_attr(feature = "pedantic", warn(clippy::pedantic))]
#![warn(clippy::use_self)]
#![warn(deprecated_in_future)]
#![warn(future_incompatible)]
#![warn(unreachable_pub)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]
#![warn(unused)]
#![deny(warnings)]
#![doc(html_root_url = "https://docs.rs/chrono-humanize/0.2.2")]

pub use crate::humantime::{Accuracy, HumanTime, Tense};

mod humantime;

/// Present the object in human friendly text form
pub trait Humanize {
    /// Emits `String` that represents current object in human friendly form
    fn humanize(&self) -> String;
}
