use crate::attributes::{StunAttribute, StunAttributeType};
use crate::common::check_buffer_boundaries;
use crate::error::{StunError, StunErrorType};
use crate::{Encode, TransactionId};
use byteorder::{BigEndian, ByteOrder};
use std::convert::{TryFrom, TryInto};

/// The message type defines the message class (request, success
/// response, error response, or indication) and the message method (the
/// primary function) of the STUN message.  Although there are four
/// message classes, there are only two types of transactions in STUN:
/// request/response transactions (which consist of a request message and
/// a response message) and indication transactions (which consist of a
/// single indication message).  Response classes are split into error
/// and success responses to aid in quickly processing the STUN message.
/// # Examples
///```rust
/// # use stun_rs::{MessageClass, MessageMethod, MessageType};
/// # use stun_rs::methods::BINDING;
/// # use std::convert::TryFrom;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// let msg_type = MessageType::new(BINDING, MessageClass::SuccessResponse);
/// assert_eq!(msg_type.as_u16(), 0x0101);
/// assert_eq!(msg_type.method(), BINDING);
/// assert_eq!(msg_type.class(), MessageClass::SuccessResponse);
/// #
/// #  Ok(())
/// # }
///```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageType {
    method: MessageMethod,
    class: MessageClass,
}

impl MessageType {
    /// Creates a new message type.
    /// # Arguments:
    /// - `method`- the message method.
    /// - `class` - The message class.
    pub fn new(method: MessageMethod, class: MessageClass) -> Self {
        Self { method, class }
    }

    /// Returns the message class.
    pub fn class(&self) -> MessageClass {
        self.class
    }

    /// Returns the message method
    pub fn method(&self) -> MessageMethod {
        self.method
    }

    /// Returns the [`u16`] representation of this [`MessageType`]
    pub fn as_u16(&self) -> u16 {
        ((self.method.0 & 0x1F80) << 2)
            | ((self.method.as_u16() & 0x0070) << 1)
            | (self.method.as_u16() & 0x000F)
            | ((self.class.as_u16() & 0x0002) << 7)
            | ((self.class.as_u16() & 0x0001) << 4)
    }
}

impl From<u16> for MessageType {
    fn from(value: u16) -> Self {
        // Discard two most significant bits
        let val = value & 0x3FFF;
        // There is not way this can fail. Value gotten will always fit into a u8
        // and it will be less or equal to 0x0003
        let class_u8: u8 = (((val & 0x0100) >> 7) | ((val & 0x0010) >> 4))
            .try_into()
            .unwrap();
        let class = MessageClass::try_from(class_u8).unwrap();
        // There is not way that method number falls out of the range defined 0x000-0xFFF
        let method_u16: u16 = ((val & 0x3E00) >> 2) | ((val & 0x00E0) >> 1) | (val & 0x000F);
        let method = MessageMethod::try_from(method_u16).unwrap();

        MessageType::new(method, class)
    }
}

impl From<&[u8; 2]> for MessageType {
    fn from(value: &[u8; 2]) -> Self {
        MessageType::from(BigEndian::read_u16(value))
    }
}

impl Encode for MessageType {
    fn encode(&self, buffer: &mut [u8]) -> Result<usize, StunError> {
        check_buffer_boundaries(buffer, 2)?;
        BigEndian::write_u16(buffer, self.as_u16());
        Ok(2)
    }
}

/// The STUN method is a 12 bits hex number in the range 0x000-0xFFF but
/// valid values are defined in the range 0x00-0xFF.
/// STUN methods in the range 0x000-0x07F are assigned by `IETF` Review
/// [`RFC8126`](https://datatracker.ietf.org/doc/html/rfc8126). STUN
/// methods in the range 0x080-0x0FF are assigned by Expert Review.
///
/// # Examples
///```rust
/// # use stun_rs::{MessageMethod, StunErrorType};
/// # use std::convert::TryFrom;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// // Create a binding method
/// let binding = MessageMethod::try_from(0x001)?;
/// assert_eq!(binding.as_u16(), 0x001);
/// // Binding request is within the range of valid values 0x00-0xFF
/// assert!(binding.is_valid());
///
/// // Create a custom method
/// let method = MessageMethod::try_from(0x100)?;
/// // This method is out of the range of valid values 0x00-0xFF
/// assert!(!method.is_valid());
///
/// // Creating a message method out of 12 bits range 0x000-0xFFF
/// // will result in an error
/// assert_eq!(MessageMethod::try_from(0x1000).expect_err("Error expected"), StunErrorType::InvalidParam);
/// #
/// #   Ok(())
/// # }
///```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MessageMethod(pub(crate) u16);

impl MessageMethod {
    /// Returns the [`u16`] representation of this message method.
    pub fn as_u16(&self) -> u16 {
        self.0
    }

    /// Returns true if the method is within the valid range 0x00-0xFF
    pub fn is_valid(&self) -> bool {
        (0x00..=0xff).contains(&self.0)
    }
}

impl TryFrom<u16> for MessageMethod {
    type Error = StunError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        (value & 0xF000 == 0)
            .then_some(MessageMethod(value))
            .ok_or_else(|| {
                StunError::new(
                    StunErrorType::InvalidParam,
                    format!("Value '{:#02x}' is not a valid a MessageMethod", value),
                )
            })
    }
}

/// The STUN message class. Although there are four
/// message classes, there are only two types of transactions in STUN:
/// request/response transactions (which consist of a request message and
/// a response message) and indication transactions (which consist of a
/// single indication message).  Response classes are split into error
/// and success responses to aid in quickly processing the STUN message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageClass {
    /// request
    Request,
    /// indication
    Indication,
    /// success response
    SuccessResponse,
    /// error response
    ErrorResponse,
}

impl MessageClass {
    fn as_u16(&self) -> u16 {
        match self {
            MessageClass::Request => 0b00,
            MessageClass::Indication => 0b01,
            MessageClass::SuccessResponse => 0b10,
            MessageClass::ErrorResponse => 0b11,
        }
    }
}

impl TryFrom<u8> for MessageClass {
    type Error = StunError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(MessageClass::Request),
            0b01 => Ok(MessageClass::Indication),
            0b10 => Ok(MessageClass::SuccessResponse),
            0b11 => Ok(MessageClass::ErrorResponse),
            _ => Err(StunError::new(
                StunErrorType::InvalidParam,
                format!("Value '{:#02x}' is not a valid a MessageClass", value),
            )),
        }
    }
}

#[derive(Debug)]
struct StunMessageParameters {
    method: MessageMethod,
    class: MessageClass,
    transaction_id: Option<TransactionId>,
    attributes: Vec<StunAttribute>,
}

/// The [`StunMessageBuilder`] ease the creation of a [`StunMessage`]
///
/// # Examples
///```rust
/// # use stun_rs::{MessageClass, MessageMethod, StunAttribute, StunMessage, StunMessageBuilder};
/// # use stun_rs::attributes::stun::{Software, UserName, Nonce};
/// # use stun_rs::methods::BINDING;
/// # use std::convert::TryFrom;
/// # use std::error::Error;
/// #
/// # fn main() -> Result<(), Box<dyn Error>> {
/// // Create a SUN request message with a random transaction ID.
/// let message = StunMessageBuilder::new(
///     BINDING,
///     MessageClass::Request,
/// )
/// .with_attribute(UserName::try_from("test-username")?)
/// .with_attribute(Software::new("test-software")?)
/// .build();
///
/// let username = message.get::<UserName>()
///   .ok_or("UserName attriute not found")?
///   .as_user_name()?;
/// assert_eq!(username, "test-username");
///
/// let software = message.get::<Software>()
///   .ok_or("Software attriute not found")?
///   .as_software()?;
/// assert_eq!(software, "test-software");
///
/// // Nonce attribute must return None
/// assert!(message.get::<Nonce>().is_none());
/// #
/// #   Ok(())
/// # }
///```
#[derive(Debug)]
pub struct StunMessageBuilder(StunMessageParameters);

impl StunMessageBuilder {
    /// Creates a new builder.
    /// # Arguments:
    /// - `method` - Message method.
    /// - `class` - Message class.
    pub fn new(method: MessageMethod, class: MessageClass) -> StunMessageBuilder {
        Self(StunMessageParameters {
            method,
            class,
            transaction_id: None,
            attributes: Vec::new(),
        })
    }

    /// Creates a STUN message using an specific transaction ID. If no
    /// [`TransactionId`] is specified, a random one will be used
    pub fn with_transaction_id(mut self, transaction_id: TransactionId) -> Self {
        self.0.transaction_id = Some(transaction_id);
        self
    }

    /// Adds an attribute to the message.
    pub fn with_attribute<T>(mut self, attribute: T) -> Self
    where
        T: Into<StunAttribute>,
    {
        self.0.attributes.push(attribute.into());
        self
    }

    /// Creates the STUN message.
    pub fn build(self) -> StunMessage {
        StunMessage {
            method: self.0.method,
            class: self.0.class,
            transaction_id: self.0.transaction_id.unwrap_or_default(),
            attributes: self.0.attributes,
        }
    }
}

/// The stun message is the basic unit of information interchanged between
/// two agents implementing the STUN protocol.
///
/// All STUN messages comprise a 20-byte header followed by zero or more
/// attributes. The STUN header contains a STUN message type, message
/// length, magic cookie, and transaction ID.
///
/// STUN messages can be created using the [`StunMessageBuilder`].
#[derive(Debug)]
pub struct StunMessage {
    method: MessageMethod,
    class: MessageClass,
    transaction_id: TransactionId,
    attributes: Vec<StunAttribute>,
}

impl StunMessage {
    /// Returns the message method.
    pub fn method(&self) -> MessageMethod {
        self.method
    }

    /// Returns the message class
    pub fn class(&self) -> MessageClass {
        self.class
    }

    /// Returns the transaction-id
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }

    /// Returns the attributes contained in this STUN message.
    pub fn attributes(&self) -> &[StunAttribute] {
        &self.attributes
    }

    /// Returns the attribute if the message contains the attribute type
    /// or None if there is no such attribute.
    /// If there are more than one attributes of this type, this function
    /// will return the first one.
    pub fn get<A>(&self) -> Option<&StunAttribute>
    where
        A: StunAttributeType,
    {
        self.attributes
            .iter()
            .find(|&attr| attr.attribute_type() == A::get_type())
    }
}

#[cfg(test)]
mod tests {
    use crate::{message::*, methods::BINDING};

    #[test]
    fn message_class() {
        let cls = MessageClass::try_from(0).expect("Can not create MessageClass");
        assert_eq!(cls.as_u16(), 0);

        let cls = MessageClass::try_from(1).expect("Can not create MessageClass");
        assert_eq!(cls.as_u16(), 1);

        let cls = MessageClass::try_from(2).expect("Can not create MessageClass");
        assert_eq!(cls.as_u16(), 2);

        let cls = MessageClass::try_from(3).expect("Can not create MessageClass");
        assert_eq!(cls.as_u16(), 3);

        MessageClass::try_from(4).expect_err("MessageClass should not be created");
    }

    #[test]
    fn message_method() {
        let m = MessageMethod::try_from(0x0000).expect("Can not create MessageMethod");
        assert_eq!(m.as_u16(), 0x0000);

        let m = MessageMethod::try_from(0x0001).expect("Can not create MessageMethod");
        assert_eq!(m.as_u16(), 0x0001);

        let m = MessageMethod::try_from(0x0FFF).expect("Can not create MessageMethod");
        assert_eq!(m.as_u16(), 0x0FFF);

        MessageMethod::try_from(0x1000).expect_err("MessageMethod should not be created");
    }

    #[test]
    fn message_type() {
        let cls = MessageClass::Request;
        let method = MessageMethod::try_from(0x0001).expect("Can not create MessageMethod");
        let msg_type = MessageType::new(method, cls);

        assert_eq!(msg_type.class(), cls);
        assert_eq!(msg_type.method(), method);

        let mut buffer: [u8; 2] = [0; 2];
        assert_eq!(msg_type.encode(&mut buffer), Ok(2));
        assert_eq!(buffer, [0x00, 0x01]);
    }

    #[test]
    fn encode_message_type() {
        let cls = MessageClass::Request;
        let method = MessageMethod::try_from(0x08D8).expect("Can not create MessageMethod");
        let msg_type = MessageType::new(method, cls);

        let mut buffer: [u8; 2] = [0; 2];
        assert_eq!(msg_type.encode(&mut buffer), Ok(2));
        assert_eq!(buffer, [0x22, 0xA8]);

        let cls = MessageClass::Indication;
        let msg_type = MessageType::new(method, cls);
        let mut buffer: [u8; 2] = [0; 2];
        assert_eq!(msg_type.encode(&mut buffer), Ok(2));
        assert_eq!(buffer, [0x22, 0xB8]);

        let cls = MessageClass::SuccessResponse;
        let msg_type = MessageType::new(method, cls);
        let mut buffer: [u8; 2] = [0; 2];
        assert_eq!(msg_type.encode(&mut buffer), Ok(2));
        assert_eq!(buffer, [0x23, 0xA8]);

        let cls = MessageClass::ErrorResponse;
        let msg_type = MessageType::new(method, cls);
        let mut buffer: [u8; 2] = [0; 2];
        assert_eq!(msg_type.encode(&mut buffer), Ok(2));
        assert_eq!(buffer, [0x23, 0xB8]);

        let cls = MessageClass::ErrorResponse;
        let msg_type = MessageType::new(method, cls);
        let mut buffer: [u8; 1] = [0; 1];
        assert_eq!(
            msg_type.encode(&mut buffer).expect_err("Error expected"),
            StunErrorType::SmallBuffer
        );
    }

    #[test]
    fn message_type_from() {
        let method = MessageMethod::try_from(0x08D8).expect("Can not create MessageMethod");

        let buffer = [0x22, 0xA8];
        let msg_type = MessageType::from(&buffer);
        assert_eq!(msg_type.class(), MessageClass::Request);
        assert_eq!(msg_type.method(), method);

        let buffer = [0x22, 0xB8];
        let msg_type = MessageType::from(&buffer);
        assert_eq!(msg_type.class(), MessageClass::Indication);
        assert_eq!(msg_type.method(), method);

        let buffer = [0x23, 0xA8];
        let msg_type = MessageType::from(&buffer);
        assert_eq!(msg_type.class(), MessageClass::SuccessResponse);
        assert_eq!(msg_type.method(), method);

        let buffer = [0x23, 0xB8];
        let msg_type = MessageType::from(&buffer);
        assert_eq!(msg_type.class(), MessageClass::ErrorResponse);
        assert_eq!(msg_type.method(), method);

        let buffer = [0x23, 0xB8];
        let msg_type = MessageType::from(&buffer);
        assert_eq!(msg_type.class(), MessageClass::ErrorResponse);
        assert_eq!(msg_type.method(), method);
    }

    #[test]
    fn fmt() {
        let cls = MessageClass::Request;
        let method = MessageMethod::try_from(0x0001).expect("Can not create MessageMethod");
        let msg_type = MessageType::new(method, cls);
        let _val = format!("{:?}", msg_type);

        let builder = StunMessageBuilder::new(BINDING, MessageClass::Request);
        let _val = format!("{:?}", builder);

        let msg = builder.build();
        let _val = format!("{:?}", msg);
    }
}
