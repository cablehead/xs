//! Implementation of a tokio-util [`Decoder`] for WebSocket
//! frames. The [`Encoder`] is a placeholder and unreachable, since tokio-util's
//! internal buffer used in the encoder comes with a hefty performance penalty
//! for large payloads due to the required memmove. Instead, we implement our
//! own, zero-copy implementation in the sink implementation of the
//! [`WebSocketStream`].
//!
//! [`WebSocketStream`]: super::WebSocketStream

use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use super::types::{Frame, Limits, OpCode, Role};
use crate::{
    mask,
    proto::ProtocolError,
    utf8::{self, Validator},
    CloseCode, Error, Payload,
};

/// Maximum size of a frame header (2 + 8 + 4).
const MAX_FRAME_HEADER_SIZE: usize = 14;

/// The actual implementation of the WebSocket byte-level protocol.
/// It provides a [`Decoder`] for single frames that must be assembled by a
/// client such as the [`WebSocketStream`] later.
///
/// [`WebSocketStream`]: super::WebSocketStream
#[derive(Debug)]
pub(super) struct WebSocketProtocol {
    /// The [`Role`] this implementation should assume for the stream.
    pub(super) role: Role,
    /// The [`Limits`] imposed on this stream.
    pub(super) limits: Limits,
    /// Opcode of the full message.
    fragmented_message_opcode: OpCode,
    /// Index up to which the payload was processed (unmasked and validated).
    payload_processed: usize,
    /// UTF-8 validator.
    validator: Validator,
}

impl WebSocketProtocol {
    /// Creates a new WebSocket codec.
    #[cfg(any(feature = "client", feature = "server"))]
    pub(super) fn new(role: Role, limits: Limits) -> Self {
        Self {
            role,
            limits,
            fragmented_message_opcode: OpCode::Continuation,
            payload_processed: 0,
            validator: Validator::new(),
        }
    }
}

/// Macro that gets a range of a buffer. It returns `Ok(None)` and reserves
/// missing capacity of the buffer if it is too small.
macro_rules! get_buf_if_space {
    ($buf:expr, $range:expr) => {
        if let Some(cont) = $buf.get($range) {
            cont
        } else {
            $buf.reserve(MAX_FRAME_HEADER_SIZE - $range.len());

            return Ok(None);
        }
    };
}

impl Decoder for WebSocketProtocol {
    type Error = Error;
    type Item = Frame;

    #[allow(clippy::cast_possible_truncation, clippy::too_many_lines)]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Opcode and payload length must be present
        let first_two_bytes = get_buf_if_space!(src, 0..2);

        // Bit 0
        let fin = first_two_bytes[0] >> 7 != 0;

        // Bits 1-3
        let rsv = first_two_bytes[0] & 0x70;

        if rsv != 0 {
            return Err(Error::Protocol(ProtocolError::InvalidRsv));
        }

        // Bits 4-7
        let opcode = OpCode::try_from(first_two_bytes[0] & 0xF)?;

        if opcode.is_control() {
            if !fin {
                return Err(Error::Protocol(ProtocolError::FragmentedControlFrame));
            }
        } else if self.fragmented_message_opcode == OpCode::Continuation {
            if opcode == OpCode::Continuation {
                return Err(Error::Protocol(ProtocolError::InvalidOpcode));
            }
        } else if opcode != OpCode::Continuation {
            return Err(Error::Protocol(ProtocolError::InvalidOpcode));
        }

        // Bit 0
        let masked = first_two_bytes[1] >> 7 != 0;

        if masked && self.role == Role::Client {
            return Err(Error::Protocol(ProtocolError::UnexpectedMaskedFrame));
        } else if !masked && self.role == Role::Server {
            return Err(Error::Protocol(ProtocolError::UnexpectedUnmaskedFrame));
        }

        // Bits 1-7
        let mut payload_length = (first_two_bytes[1] & 127) as usize;

        let mut offset = 2;

        // Close frames must be at least 2 bytes in length
        if opcode == OpCode::Close && payload_length == 1 {
            return Err(Error::Protocol(ProtocolError::InvalidPayloadLength));
        } else if payload_length > 125 {
            if opcode.is_control() {
                return Err(Error::Protocol(ProtocolError::InvalidPayloadLength));
            }

            if payload_length == 126 {
                // A conversion from 2 u8s to a u16 cannot fail
                payload_length =
                    u16::from_be_bytes(get_buf_if_space!(src, 2..4).try_into().unwrap()) as usize;
                if payload_length <= 125 {
                    return Err(Error::Protocol(ProtocolError::InvalidPayloadLength));
                }
                offset = 4;
            } else if payload_length == 127 {
                // A conversion from 8 u8s to a u64 cannot fail
                payload_length =
                    u64::from_be_bytes(get_buf_if_space!(src, 2..10).try_into().unwrap()) as usize;
                if u16::try_from(payload_length).is_ok() {
                    return Err(Error::Protocol(ProtocolError::InvalidPayloadLength));
                }
                offset = 10;
            } else {
                debug_assert!(false, "7 bit value expected to be <= 127");
            }
        }

        if payload_length > self.limits.max_payload_len {
            return Err(Error::PayloadTooLong {
                len: payload_length,
                max_len: self.limits.max_payload_len,
            });
        }

        if masked {
            offset += 4;
            if src.len() < offset {
                src.reserve(MAX_FRAME_HEADER_SIZE - 4);

                return Ok(None);
            }
        }

        if payload_length != 0 {
            let is_text = opcode == OpCode::Text
                || (opcode == OpCode::Continuation
                    && self.fragmented_message_opcode == OpCode::Text);
            let payload_available = (src.len() - offset).min(payload_length);
            let is_complete = payload_available == payload_length;

            let payload = if masked && (is_complete || is_text) {
                let (l, r) = unsafe { src.split_at_mut_unchecked(offset) };
                let mask = unsafe { l.get_unchecked_mut(l.len() - 4..).try_into().unwrap() };
                // SAFETY: self.payload_processed <= payload_length
                let payload =
                    unsafe { r.get_unchecked_mut(self.payload_processed..payload_available) };

                mask::frame(mask, payload);

                payload
            } else {
                // SAFETY: self.payload_processed <= payload_length
                unsafe {
                    src.get_unchecked_mut(
                        offset + self.payload_processed..offset + payload_available,
                    )
                }
            };

            if is_text {
                self.validator.feed(payload, is_complete && fin)?;

                self.payload_processed = payload_available;
            }

            if !is_complete {
                src.reserve(payload_length - payload_available);

                return Ok(None);
            }

            if opcode == OpCode::Close {
                // SAFETY: Close frames with a non-zero payload length are validated to not have
                // a length of 1
                // A conversion from two u8s to a u16 cannot fail
                let code = CloseCode::try_from(u16::from_be_bytes(unsafe {
                    src.get_unchecked(offset..offset + 2).try_into().unwrap()
                }))?;
                if code.is_reserved() {
                    return Err(Error::Protocol(ProtocolError::InvalidCloseCode));
                }

                // SAFETY: payload_length <= src.len()
                let _reason = utf8::parse_str(unsafe {
                    src.get_unchecked(offset + 2..offset + payload_length)
                })?;
            }
        }

        // Advance the offset into the payload body
        src.advance(offset);
        // Take the payload
        let mut payload = Payload::from(src.split_to(payload_length));
        payload.set_utf8_validated(opcode == OpCode::Text && fin);

        // It is possible to receive intermediate control frames between a large other
        // frame. We therefore can't simply reset the fragmented opcode after we receive
        // a "final" frame.
        if (fin && opcode == OpCode::Continuation) || (!fin && opcode != OpCode::Continuation) {
            // Full chunked message received (and opcode is Continuation)
            // or first frame of a multi-frame message received
            self.fragmented_message_opcode = opcode;
        }
        // In all other cases, we have either a continuation or control frame, neither
        // of which change change the opcode being assembled

        self.payload_processed = 0;

        Ok(Some(Frame {
            opcode,
            payload,
            is_final: fin,
        }))
    }
}
