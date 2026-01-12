//! Types required for the WebSocket protocol implementation.
use std::{fmt, mem::replace, num::NonZeroU16, ops::Deref};

use bytes::{BufMut, Bytes, BytesMut};

use super::error::ProtocolError;
use crate::utf8;

/// The opcode of a WebSocket frame. It denotes the type of the frame or an
/// assembled message.
///
/// A fully assembled [`Message`] will never have a continuation opcode.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(super) enum OpCode {
    /// A continuation opcode. This will never be encountered in a full
    /// [`Message`].
    Continuation,
    /// A text opcode.
    Text,
    /// A binary opcode.
    Binary,
    /// A close opcode.
    Close,
    /// A ping opcode.
    Ping,
    /// A pong opcode.
    Pong,
}

impl OpCode {
    /// Whether this is a control opcode (i.e. close, ping or pong).
    pub(super) fn is_control(self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }
}

impl TryFrom<u8> for OpCode {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Continuation),
            1 => Ok(Self::Text),
            2 => Ok(Self::Binary),
            8 => Ok(Self::Close),
            9 => Ok(Self::Ping),
            10 => Ok(Self::Pong),
            _ => Err(ProtocolError::InvalidOpcode),
        }
    }
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> Self {
        match value {
            OpCode::Continuation => 0,
            OpCode::Text => 1,
            OpCode::Binary => 2,
            OpCode::Close => 8,
            OpCode::Ping => 9,
            OpCode::Pong => 10,
        }
    }
}

/// Close status code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CloseCode(NonZeroU16);

// rustfmt reorders these alphabetically
#[rustfmt::skip]
impl CloseCode {
    /// Normal closure, meaning that the purpose for which the connection was
    /// established has been fulfilled.
    pub const NORMAL_CLOSURE: Self = Self::constant(1000);
    /// Endpoint is "going away", such as a server going down or a browser
    /// having navigated away from a page.
    pub const GOING_AWAY: Self = Self::constant(1001);
    /// Endpoint is terminating the connection due to a protocol error.
    pub const PROTOCOL_ERROR: Self = Self::constant(1002);
    /// Endpoint is terminating the connection because it has received a type of
    /// data it cannot accept.
    pub const UNSUPPORTED_DATA: Self = Self::constant(1003);
    /// No status code was actually present.
    pub const NO_STATUS_RECEIVED: Self = Self::constant(1005);
    /// Endpoint is terminating the connection because it has received data
    /// within a message that was not consistent with the type of the message.
    pub const INVALID_FRAME_PAYLOAD_DATA: Self = Self::constant(1007);
    /// Endpoint is terminating the connection because it has received a message
    /// that violates its policy.
    pub const POLICY_VIOLATION: Self = Self::constant(1008);
    /// Endpoint is terminating the connection because it has received a message
    /// that is too big for it to process.
    pub const MESSAGE_TOO_BIG: Self = Self::constant(1009);
    /// Client is terminating the connection because it has expected the server
    /// to negotiate one or more extension, but the server didn't return them in
    /// the response message of the WebSocket handshake.
    pub const MANDATORY_EXTENSION: Self = Self::constant(1010);
    /// Server is terminating the connection because it encountered an
    /// unexpected condition that prevented it from fulfilling the request.
    pub const INTERNAL_SERVER_ERROR: Self = Self::constant(1011);
    /// Service is restarted. A client may reconnect, and if it choses to do,
    /// should reconnect using a randomized delay of 5--30s.
    pub const SERVICE_RESTART: Self = Self::constant(1012);
    /// Service is experiencing overload. A client should only connect to a
    /// different IP (when there are multiple for the target) or reconnect to
    /// the same IP upon user action.
    pub const SERVICE_OVERLOAD: Self = Self::constant(1013);
    /// The server was acting as a gateway or proxy and received an invalid
    /// response from the upstream server. This is similar to the HTTP 502
    /// status code.
    pub const BAD_GATEWAY: Self = Self::constant(1014);
}

impl CloseCode {
    /// Try to construct [`CloseCode`] from `u16`
    ///
    /// Returns `None` if `code` is not a valid `CloseCode`
    const fn try_from_u16(code: u16) -> Option<Self> {
        match code {
            1000..=1015 | 3000..=4999 => {
                // FIXME: replace with `Some(Self(NonZeroU16::new(code).unwrap()))`
                // once MSRV is bumped to 1.83
                match NonZeroU16::new(code) {
                    Some(code) => Some(Self(code)),
                    None => unreachable!(),
                }
            }
            0..=999 | 1016..=2999 | 5000..=u16::MAX => None,
        }
    }

    /// Try to construct [`CloseCode`] from `u16`
    ///
    /// Panics if `code` is not a valid `CloseCode`
    const fn constant(code: u16) -> Self {
        // FIXME: replace with `Self::try_from_u16(code).unwrap()`
        // once MSRV is bumped to 1.83
        match Self::try_from_u16(code) {
            Some(code) => code,
            None => unreachable!(),
        }
    }

    /// Whether the close code is reserved and cannot be sent over the wire.
    #[must_use]
    pub fn is_reserved(self) -> bool {
        match self.0.get() {
            1004 | 1005 | 1006 | 1015 => true,
            1000..=4999 => false,
            // `TryFrom` is the only way to acquire self and it errors for these values
            0..=999 | 5000..=u16::MAX => {
                debug_assert!(false, "unexpected CloseCode");
                false
            }
        }
    }
}

impl From<CloseCode> for u16 {
    fn from(value: CloseCode) -> Self {
        value.0.get()
    }
}

impl TryFrom<u16> for CloseCode {
    type Error = ProtocolError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::try_from_u16(value).ok_or(ProtocolError::InvalidCloseCode)
    }
}

/// The websocket message payload storage.
///
/// Payloads can be created by using the `From<T>` implementations.
///
/// Sending the payloads or calling [`Into<BytesMut>`] is zero-copy, except when
/// sending a payload created from a static slice or when the payload buffer is
/// not unique. All conversions to other types are zero-cost.
///
/// [`Into<BytesMut>`]: #impl-From<Payload>-for-BytesMut
#[derive(Clone)]
pub struct Payload {
    /// The raw payload data.
    data: Bytes,
    /// Whether the payload data was validated to be valid UTF-8.
    utf8_validated: bool,
}

impl Payload {
    /// Creates a new shared `Payload` from a static slice.
    const fn from_static(bytes: &'static [u8]) -> Self {
        Self {
            data: Bytes::from_static(bytes),
            utf8_validated: false,
        }
    }

    /// Marks whether the payload contents were validated to be valid UTF-8.
    pub(super) fn set_utf8_validated(&mut self, value: bool) {
        self.utf8_validated = value;
    }

    /// Shortens the buffer, keeping the first `len` bytes and dropping the
    /// rest.
    pub(super) fn truncate(&mut self, len: usize) {
        self.data.truncate(len);
    }

    /// Splits the buffer into two at the given index.
    fn split_to(&mut self, at: usize) -> Self {
        // This is only used by the outgoing message frame iterator, so we do not care
        // about the value of utf8_validated. For the sake of correctness (in case we
        // split a utf8 codepoint), we set it to false.
        self.utf8_validated = false;
        Self {
            data: self.data.split_to(at),
            utf8_validated: false,
        }
    }
}

impl Deref for Payload {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl fmt::Debug for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Payload").field(&self.data).finish()
    }
}

impl From<Bytes> for Payload {
    fn from(value: Bytes) -> Self {
        Self {
            data: value,
            utf8_validated: false,
        }
    }
}

impl From<BytesMut> for Payload {
    fn from(value: BytesMut) -> Self {
        Self {
            data: value.freeze(),
            utf8_validated: false,
        }
    }
}

impl From<Payload> for Bytes {
    fn from(value: Payload) -> Self {
        value.data
    }
}

impl From<Payload> for BytesMut {
    fn from(value: Payload) -> Self {
        value.data.into()
    }
}

impl From<Vec<u8>> for Payload {
    fn from(value: Vec<u8>) -> Self {
        // BytesMut::from_iter goes through a specialization in std if the iterator is a
        // Vec, effectively allowing us to use BytesMut::from_vec which isn't
        // exposed in bytes. See https://github.com/tokio-rs/bytes/issues/723 for details.
        Self {
            data: BytesMut::from_iter(value).freeze(),
            utf8_validated: false,
        }
    }
}

impl From<String> for Payload {
    fn from(value: String) -> Self {
        // See From<Vec<u8>> impl for reasoning behind this.
        Self {
            data: BytesMut::from_iter(value.into_bytes()).freeze(),
            utf8_validated: true,
        }
    }
}

impl From<&'static [u8]> for Payload {
    fn from(value: &'static [u8]) -> Self {
        Self {
            data: Bytes::from_static(value),
            utf8_validated: false,
        }
    }
}

impl From<&'static str> for Payload {
    fn from(value: &'static str) -> Self {
        Self {
            data: Bytes::from_static(value.as_bytes()),
            utf8_validated: true,
        }
    }
}

/// A WebSocket message. This is cheaply clonable and uses [`Payload`] as the
/// payload storage underneath.
///
/// Received messages are always validated prior to dealing with them, so all
/// the type casting methods are either almost or fully zero cost.
#[derive(Debug, Clone)]
pub struct Message {
    /// The [`OpCode`] of the message.
    pub(super) opcode: OpCode,
    /// The payload of the message.
    pub(super) payload: Payload,
}

impl Message {
    /// Create a new text message. The payload contents must be valid UTF-8.
    #[must_use]
    pub fn text<P: Into<Payload>>(payload: P) -> Self {
        Self {
            opcode: OpCode::Text,
            payload: payload.into(),
        }
    }

    /// Create a new binary message.
    #[must_use]
    pub fn binary<P: Into<Payload>>(payload: P) -> Self {
        Self {
            opcode: OpCode::Binary,
            payload: payload.into(),
        }
    }

    /// Create a new close message. If an non-empty reason is specified, a
    /// [`CloseCode`] must be specified for it to be included.
    ///
    /// # Panics
    /// - If the `code` is reserved so it cannot be sent.
    /// - If `code` is present and the `reason` exceeds 123 bytes, the
    ///   protocol-imposed limit.
    #[must_use]
    #[track_caller]
    pub fn close(code: Option<CloseCode>, reason: &str) -> Self {
        let mut payload = BytesMut::with_capacity((2 + reason.len()) * usize::from(code.is_some()));

        if let Some(code) = code {
            assert!(!code.is_reserved());
            payload.put_u16(code.into());

            assert!(reason.len() <= 123);
            payload.extend_from_slice(reason.as_bytes());
        }

        Self {
            opcode: OpCode::Close,
            payload: payload.into(),
        }
    }

    /// Create a new ping message.
    ///
    /// # Panics
    /// If the payload exceeds 125 bytes, the protocol-imposed limit.
    #[must_use]
    #[track_caller]
    pub fn ping<P: Into<Payload>>(payload: P) -> Self {
        let payload = payload.into();
        assert!(payload.len() <= 125);
        Self {
            opcode: OpCode::Ping,
            payload,
        }
    }

    /// Create a new pong message.
    ///
    /// # Panics
    /// If the payload exceeds 125 bytes, the protocol-imposed limit.
    #[must_use]
    #[track_caller]
    pub fn pong<P: Into<Payload>>(payload: P) -> Self {
        let payload = payload.into();
        assert!(payload.len() <= 125);
        Self {
            opcode: OpCode::Pong,
            payload,
        }
    }

    /// Whether the message is a text message.
    #[must_use]
    pub fn is_text(&self) -> bool {
        self.opcode == OpCode::Text
    }

    /// Whether the message is a binary message.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        self.opcode == OpCode::Binary
    }

    /// Whether the message is a close message.
    #[must_use]
    pub fn is_close(&self) -> bool {
        self.opcode == OpCode::Close
    }

    /// Whether the message is a ping message.
    #[must_use]
    pub fn is_ping(&self) -> bool {
        self.opcode == OpCode::Ping
    }

    /// Whether the message is a pong message.
    #[must_use]
    pub fn is_pong(&self) -> bool {
        self.opcode == OpCode::Pong
    }

    /// Returns the message payload and consumes the message, regardless of
    /// type.
    #[must_use]
    pub fn into_payload(self) -> Payload {
        self.payload
    }

    /// Returns a reference to the message payload, regardless of message type.
    pub fn as_payload(&self) -> &Payload {
        &self.payload
    }

    /// Returns a reference to the message payload as a string if it is a text
    /// message.
    ///
    /// # Panics
    ///
    /// This method will panic when the message was created via
    /// [`Message::text`] with invalid UTF-8.
    pub fn as_text(&self) -> Option<&str> {
        // SAFETY: Received messages were validated to be valid UTF-8, otherwise
        // we check if it is valid UTF-8.
        (self.opcode == OpCode::Text).then(|| {
            assert!(
                self.payload.utf8_validated || utf8::parse_str(&self.payload).is_ok(),
                "called as_text on message created from payload with invalid utf-8"
            );
            unsafe { std::str::from_utf8_unchecked(&self.payload) }
        })
    }

    /// Returns the [`CloseCode`] and close reason if the message is a close
    /// message.
    pub fn as_close(&self) -> Option<(CloseCode, &str)> {
        (self.opcode == OpCode::Close).then(|| {
            let code = if self.payload.is_empty() {
                CloseCode::NO_STATUS_RECEIVED
            } else {
                // SAFETY: Opcode is Close with a non-empty payload so it's at least 2 bytes
                // long
                unsafe {
                    CloseCode::try_from(u16::from_be_bytes(
                        self.payload
                            .get_unchecked(0..2)
                            .try_into()
                            .unwrap_unchecked(),
                    ))
                    .unwrap_unchecked()
                }
            };

            // SAFETY: Opcode is Close so the rest of the payload is valid UTF-8
            let reason =
                unsafe { std::str::from_utf8_unchecked(self.payload.get(2..).unwrap_or_default()) };

            (code, reason)
        })
    }

    /// Returns an iterator over frames of `frame_size` length to split this
    /// message into.
    pub(super) fn into_frames(self, frame_size: usize) -> MessageFrames {
        MessageFrames {
            frame_size,
            payload: self.payload,
            opcode: self.opcode,
        }
    }
}

/// Iterator over frames of a chunked message.
pub(super) struct MessageFrames {
    /// Iterator over payload chunks.
    frame_size: usize,
    /// The full message payload this iterates over.
    payload: Payload,
    /// Opcode for the next frame.
    opcode: OpCode,
}

impl Iterator for MessageFrames {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let is_empty = self.payload.is_empty() && self.opcode == OpCode::Continuation;

        (!is_empty).then(|| {
            let payload = self
                .payload
                .split_to(self.frame_size.min(self.payload.len()));

            Frame {
                opcode: replace(&mut self.opcode, OpCode::Continuation),
                is_final: self.payload.is_empty(),
                payload,
            }
        })
    }
}

/// Configuration for limitations on reading of [`Message`]s from a
/// [`WebSocketStream`] to prevent high memory usage caused by malicious actors.
///
/// [`WebSocketStream`]: super::WebSocketStream
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// The maximum allowed payload length. The default is 64 MiB.
    pub(super) max_payload_len: usize,
}

impl Limits {
    /// A limit configuration without any limits.
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_payload_len: usize::MAX,
        }
    }

    /// Sets the maximum allowed payload length. `None` equals no limit.
    ///
    /// The default is 64 MiB.
    #[must_use]
    pub fn max_payload_len(mut self, size: Option<usize>) -> Self {
        self.set_max_payload_len(size);

        self
    }

    /// See [`max_payload_len`](Self::max_payload_len).
    pub fn set_max_payload_len(&mut self, size: Option<usize>) {
        self.max_payload_len = size.unwrap_or(usize::MAX);
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_payload_len: 64 * 1024 * 1024,
        }
    }
}

/// Low-level configuration for a [`WebSocketStream`] that allows configuring
/// behavior for sending and receiving messages.
///
/// [`WebSocketStream`]: super::WebSocketStream
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Frame payload size to split outgoing messages into.
    ///
    /// Consider decreasing this if the remote imposes a limit on the frame
    /// payload size. The default is 4MiB.
    pub(super) frame_size: usize,
    /// Threshold of queued up bytes after which the underlying I/O is flushed
    /// before the sink is declared ready. The default is 8 KiB.
    pub(super) flush_threshold: usize,
}

impl Config {
    /// Set the frame payload size to split outgoing messages into.
    ///
    /// Consider decreasing this if the remote imposes a limit on the frame
    /// payload size. The default is 4MiB.
    ///
    /// # Panics
    ///
    /// If `frame_size` is `0`.
    #[must_use]
    pub fn frame_size(mut self, frame_size: usize) -> Self {
        assert_ne!(frame_size, 0, "frame_size must be non-zero");
        self.frame_size = frame_size;

        self
    }

    /// Sets the threshold of queued up bytes after which the underlying I/O is
    /// flushed before the sink is declared ready. The default is 8 KiB.
    #[must_use]
    pub fn flush_threshold(mut self, threshold: usize) -> Self {
        self.flush_threshold = threshold;

        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            frame_size: 4 * 1024 * 1024,
            flush_threshold: 8 * 1024,
        }
    }
}

/// Role assumed by the [`WebSocketStream`] in a connection.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Role {
    /// The client end.
    Client,
    /// The server end.
    Server,
}

/// The connection state of the stream.
#[derive(Debug, PartialEq)]
pub(super) enum StreamState {
    /// The connection is fully active and no close has been initiated.
    Active,
    /// The connection has been closed by the peer, but not yet acknowledged by
    /// us.
    ClosedByPeer,
    /// The connection has been closed by us, but not yet acknowledged.
    ClosedByUs,
    /// The close has been acknowledged by the end that did not initiate the
    /// close.
    CloseAcknowledged,
}

/// A frame of a WebSocket [`Message`].
#[derive(Clone, Debug)]
pub(super) struct Frame {
    /// The [`OpCode`] of the frame.
    pub opcode: OpCode,
    /// Whether this is the last frame of a message.
    pub is_final: bool,
    /// The payload bytes of the frame.
    pub payload: Payload,
}

impl Frame {
    /// Default close frame.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const DEFAULT_CLOSE: Self = Self {
        opcode: OpCode::Close,
        is_final: true,
        payload: Payload::from_static(&CloseCode::NORMAL_CLOSURE.0.get().to_be_bytes()),
    };

    /// Encode the frame head into `out`, returning a subslice where the mask
    /// should be written to.
    pub fn encode<'a>(&self, out: &'a mut [u8; 14]) -> &'a mut [u8; 4] {
        out[0] = (u8::from(self.is_final) << 7) | u8::from(self.opcode);
        let mask_slice = if u16::try_from(self.payload.len()).is_err() {
            out[1] = 127;
            let len = u64::try_from(self.payload.len()).unwrap();
            out[2..10].copy_from_slice(&len.to_be_bytes());
            &mut out[10..14]
        } else if self.payload.len() > 125 {
            out[1] = 126;
            let len = u16::try_from(self.payload.len()).expect("checked by previous branch");
            out[2..4].copy_from_slice(&len.to_be_bytes());
            &mut out[4..8]
        } else {
            out[1] = u8::try_from(self.payload.len()).expect("checked by previous branch");
            &mut out[2..6]
        };
        mask_slice.try_into().unwrap()
    }
}

impl From<Message> for Frame {
    fn from(value: Message) -> Self {
        Self {
            opcode: value.opcode,
            is_final: true,
            payload: value.payload,
        }
    }
}

impl From<&ProtocolError> for Frame {
    fn from(val: &ProtocolError) -> Self {
        match val {
            ProtocolError::InvalidUtf8 => {
                Message::close(Some(CloseCode::INVALID_FRAME_PAYLOAD_DATA), "invalid utf8")
            }
            _ => Message::close(Some(CloseCode::PROTOCOL_ERROR), val.as_str()),
        }
        .into()
    }
}
