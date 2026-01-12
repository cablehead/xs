//! STUN Methods Registry

#![allow(dead_code)]

use crate::message::MessageMethod;

/// Reserved
pub const RESERVED: MessageMethod = MessageMethod(0x0000);

/// Binding
pub const BINDING: MessageMethod = MessageMethod(0x0001);

/// Shared secret
pub const SHARED_SECRET: MessageMethod = MessageMethod(0x0002);

#[cfg(feature = "turn")]
/// Allocate
pub const ALLOCATE: MessageMethod = MessageMethod(0x0003);

#[cfg(feature = "turn")]
/// Refresh
pub const REFRESH: MessageMethod = MessageMethod(0x0004);

#[cfg(feature = "turn")]
/// Send
pub const SEND: MessageMethod = MessageMethod(0x0006);

#[cfg(feature = "turn")]
/// Data
pub const DATA: MessageMethod = MessageMethod(0x0007);

#[cfg(feature = "turn")]
/// Create permission
pub const CREATE_PERMMISSION: MessageMethod = MessageMethod(0x0008);

#[cfg(feature = "turn")]
/// Channel bind
pub const CHANNEL_BIND: MessageMethod = MessageMethod(0x0009);
