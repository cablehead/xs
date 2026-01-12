pub trait HmacSha {
    fn hmac_sha(key: &[u8], message: &[u8]) -> Vec<u8>;
}

macro_rules! message_integrity_attribute {
    (
        $(#[$meta:meta])*
        $attr_class:ident,
        $attr_type:ident,
        $attr_size:ident
    ) => (
        paste::paste! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct [<Encodable $attr_class>](crate::types::HMACKey);

            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct [<Decodable $attr_class>]([u8; $attr_size]);

            impl [<Decodable $attr_class>] {
                fn validate(&self, input: &[u8], key: &crate::types::HMACKey) -> bool {
                    let expected = $attr_class::hmac_sha(key.as_bytes(), input);
                    expected == self.0
                }
            }

            impl<'a> crate::Decode<'a> for [<Decodable $attr_class>] {
                fn decode(buffer: &[u8]) -> Result<(Self, usize), crate::StunError> {
                    crate::common::check_buffer_boundaries(buffer, $attr_size)?;
                    let hmac_sha1: [u8; $attr_size] = buffer.try_into()?;
                    Ok(([<Decodable $attr_class>](hmac_sha1), $attr_size))
                }
            }

            $(#[$meta])*
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub enum $attr_class {
                /// Encodable version of this attribute. This is used when the attribute is added to a STUN message that is going to be sent to the network.
                Encodable([<Encodable $attr_class>]),
                /// Decodable version of this attribute. This is the decoded attribute received from the network.
                Decodable([<Decodable $attr_class>]),
            }

            impl $attr_class {
                /// Creates a new attribute.
                /// # Arguments:
                /// - `key` - The key used for the `HMAC` depends on which credential mechanism is in use.
                pub fn new(key: crate::types::HMACKey) -> Self {
                    $attr_class::Encodable([<Encodable $attr_class>](key))
                }

                #[doc = "Validates the message using the `HMAC` value generated from the key"]
                #[doc = "# Arguments:"]
                #[doc = "* `input`- the STUN message up to (but excluding) the [`" $attr_class "`] attribute itself."]
                #[doc = "* `key`- the [`HMACKey`](crate::types::HMACKey) key"]
                #[doc = "# Returns:"]
                #[doc = "true if the message integrity attribute matches the computed value."]
                pub fn validate(&self, input: &[u8], key: &crate::types::HMACKey) -> bool {
                    match self {
                        $attr_class::Decodable(attr) => attr.validate(input, key),
                        $attr_class::Encodable(_) => false,
                    }
                }
            }

            impl From<&[u8; $attr_size]> for $attr_class {
                fn from(val: &[u8; $attr_size]) -> Self {
                    $attr_class::Decodable([<Decodable $attr_class>](*val))
                }
            }

            impl From<[u8; $attr_size]> for $attr_class {
                fn from(val: [u8; $attr_size]) -> Self {
                    $attr_class::Decodable([<Decodable $attr_class>](val))
                }
            }

            impl crate::attributes::EncodeAttributeValue for $attr_class {
                fn encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<usize, crate::StunError> {
                    match self {
                        $attr_class::Encodable(_) => {
                            crate::common::check_buffer_boundaries(ctx.raw_value(), $attr_size)?;
                            let raw_value = ctx.raw_value_mut();
                            raw_value[0..$attr_size]
                                .iter_mut()
                                .for_each(|v| *v = 0);
                            Ok($attr_size)
                        }
                        _ => Err(crate::error::StunError::new(
                            crate::error::StunErrorType::InvalidParam,
                            format!("Not encodable attribute"),
                        )),
                    }
                }

                fn post_encode(&self, mut ctx: crate::context::AttributeEncoderContext) -> Result<(), crate::StunError> {
                    match self {
                        $attr_class::Encodable(attr) => {
                            crate::common::check_buffer_boundaries(ctx.raw_value(), $attr_size)?;
                            let hmac_sha = $attr_class::hmac_sha(attr.0.as_bytes(), ctx.encoded_message());
                            let raw_value = ctx.raw_value_mut();
                            raw_value[..$attr_size].copy_from_slice(&hmac_sha);
                            Ok(())
                        }
                        _ => Err(crate::error::StunError::new(
                            crate::error::StunErrorType::InvalidParam,
                            format!("Not encodable attribute"),
                        )),
                    }
                }
            }

            impl crate::attributes::DecodeAttributeValue for $attr_class {
                fn decode(ctx: crate::context::AttributeDecoderContext) -> Result<(Self, usize), crate::StunError> {
                    let (val, size) = [<Decodable $attr_class>]::decode(ctx.raw_value())?;
                    Ok(($attr_class::Decodable(val), size))
                }
            }
        } // paste

        impl crate::attributes::Verifiable for $attr_class {
            fn verify(&self, input: &[u8], ctx: &crate::DecoderContext) -> bool {
                match ctx.key() {
                    Some(k) => {
                        self.validate(input, k)
                    }
                    None => {
                        // HMACKey required for validation
                        false
                    }
                }
            }
        }

        impl crate::attributes::AsVerifiable for $attr_class {
            fn as_verifiable_ref(&self) -> Option<&dyn crate::attributes::Verifiable> {
                Some(self)
            }
        }

        crate::attributes::stunt_attribute!($attr_class, $attr_type);
    )
}

pub(crate) use message_integrity_attribute;
