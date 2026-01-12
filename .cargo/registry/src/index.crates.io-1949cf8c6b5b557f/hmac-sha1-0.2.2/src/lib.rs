use sha1::Sha1;
use hmac::{Hmac, Mac};

// define hash constants
pub const SHA1_DIGEST_BYTES: usize = 20;

pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; SHA1_DIGEST_BYTES] {
    // Create the hasher with the key. We can use expect for Hmac algorithms as they allow arbitrary key sizes.
    let mut hasher: Hmac<Sha1> = Mac::new_from_slice(key)
         .expect("HMAC algoritms can take keys of any size");

    // hash the message
    hasher.update(message);

    // finalize the hash and convert to a static array
    hasher.finalize().into_bytes().into()
}



#[cfg(test)]
mod tests {
    use crate::*;

    use hex::ToHex;


    #[test]
    fn test_vector1() {
        // tuples of (data, key, expected hex string)
        let data = "Hi There".as_bytes();
        let key = &[0x0b; 20];
        let expected = "b617318655057264e28bc0b6fb378c8ef146be00".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector2() {
        // tuples of (data, key, expected hex string)
        let data = "what do ya want for nothing?".as_bytes();
        let key = "Jefe".as_bytes();
        let expected = "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector3() {
        // tuples of (data, key, expected hex string)
        let data = &[0xdd; 50];
        let key = &[0xaa; 20];
        let expected = "125d7342b9ac11cd91a39af48aa17b4f63f175d3".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector4() {
        // tuples of (data, key, expected hex string)
        let data = &[0xcd; 50];
        let key = &[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25];
        let expected = "4c9007f4026250c6bc8414f9bf50c86c2d7235da".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector5() {
        // tuples of (data, key, expected hex string)
        let data = "Test With Truncation".as_bytes();
        let key = &[0x0c;20];
        let expected = "4c1a03424b55e07fe7f27be1d58bb9324a9a5a04".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector6() {
        // tuples of (data, key, expected hex string)
        let data = "Test Using Larger Than Block-Size Key - Hash Key First".as_bytes();
        let key = &[0xaa;80];
        let expected = "aa4ae5e15272d00e95705637ce8a3b55ed402112".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }

    #[test]
    fn test_vector7() {
        // tuples of (data, key, expected hex string)
        let data = "Test Using Larger Than Block-Size Key and Larger Than One Block-Size Data".as_bytes();
        let key = &[0xaa;80];
        let expected = "e8e99d0f45237d786d6bbaa7965c7808bbff1a91".to_string();

        let hash = hmac_sha1(key, data);
        assert_eq!(hash.encode_hex::<String>(),expected);
    }
}
