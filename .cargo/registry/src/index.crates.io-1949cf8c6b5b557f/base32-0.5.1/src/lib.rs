#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate quickcheck;

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::min;

#[derive(Copy, Clone)]
pub enum Alphabet {
    Crockford,
    Rfc4648 { padding: bool },
    Rfc4648Lower { padding: bool },
    Rfc4648Hex { padding: bool },
    Rfc4648HexLower { padding: bool },
    Z,
}

const CROCKFORD: &'static [u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const RFC4648: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const RFC4648_LOWER: &'static [u8] = b"abcdefghijklmnopqrstuvwxyz234567";
const RFC4648_HEX: &'static [u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";
const RFC4648_HEX_LOWER: &'static [u8] = b"0123456789abcdefghijklmnopqrstuv";
const Z: &'static [u8] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

pub fn encode(alphabet: Alphabet, data: &[u8]) -> String {
    let (alphabet, padding) = match alphabet {
        Alphabet::Crockford => (CROCKFORD, false),
        Alphabet::Rfc4648 { padding } => (RFC4648, padding),
        Alphabet::Rfc4648Lower { padding } => (RFC4648_LOWER, padding),
        Alphabet::Rfc4648Hex { padding } => (RFC4648_HEX, padding),
        Alphabet::Rfc4648HexLower { padding } => (RFC4648_HEX_LOWER, padding),
        Alphabet::Z => (Z, false),
    };
    let mut ret = Vec::with_capacity((data.len() + 3) / 4 * 5);

    for chunk in data.chunks(5) {
        let buf = {
            let mut buf = [0u8; 5];
            for (i, &b) in chunk.iter().enumerate() {
                buf[i] = b;
            }
            buf
        };
        ret.push(alphabet[((buf[0] & 0xF8) >> 3) as usize]);
        ret.push(alphabet[(((buf[0] & 0x07) << 2) | ((buf[1] & 0xC0) >> 6)) as usize]);
        ret.push(alphabet[((buf[1] & 0x3E) >> 1) as usize]);
        ret.push(alphabet[(((buf[1] & 0x01) << 4) | ((buf[2] & 0xF0) >> 4)) as usize]);
        ret.push(alphabet[(((buf[2] & 0x0F) << 1) | (buf[3] >> 7)) as usize]);
        ret.push(alphabet[((buf[3] & 0x7C) >> 2) as usize]);
        ret.push(alphabet[(((buf[3] & 0x03) << 3) | ((buf[4] & 0xE0) >> 5)) as usize]);
        ret.push(alphabet[(buf[4] & 0x1F) as usize]);
    }

    if data.len() % 5 != 0 {
        let len = ret.len();
        let num_extra = 8 - (data.len() % 5 * 8 + 4) / 5;
        if padding {
            for i in 1..num_extra + 1 {
                ret[len - i] = b'=';
            }
        } else {
            ret.truncate(len - num_extra);
        }
    }

    String::from_utf8(ret).unwrap()
}

/*
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9,  :,  ;,  <,  =,  >,  ?,  @,  A,  B,  C,
     D,  E,  F,  G,  H,  I,  J,  K,  L,  M,  N,  O,  P,  Q,  R,  S,  T,  U,  V,  W,
     X,  Y,  Z,  [,  \,  ],  ^,  _,  `,  a,  b,  c,  d,  e,  f,  g,  h,  i,  j,  k,
     l,  m,  n,  o,  p,  q,  r,  s,  t,  u,  v,  w,  x,  y,  z,
*/

const CROCKFORD_INV: [i8; 75] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12,
    13, 14, 15, 16, 17,  1, 18, 19,  1, 20, 21,  0, 22, 23, 24, 25, 26, -1, 27, 28,
    29, 30, 31, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17,  1, 18, 19,
     1, 20, 21,  0, 22, 23, 24, 25, 26, -1, 27, 28, 29, 30, 31,
];
const RFC4648_INV: [i8; 75] = [
    -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, -1, -1, -1, -1,  0,  1,  2,
     3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
    23, 24, 25, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
const RFC4648_INV_PAD: [i8; 75] = [
    -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1,  0, -1, -1, -1,  0,  1,  2,
     3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
    23, 24, 25, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
const RFC4648_INV_LOWER: [i8; 75] = [
    -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
    11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
];
const RFC4648_INV_LOWER_PAD: [i8; 75] = [
    -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1,  0, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10,
    11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
];
const RFC4648_INV_HEX: [i8; 75] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12,
    13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
const RFC4648_INV_HEX_PAD: [i8; 75] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1,  0, -1, -1, -1, 10, 11, 12,
    13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
const RFC4648_INV_HEX_LOWER: [i8; 75] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1,
];
const RFC4648_INV_HEX_LOWER_PAD: [i8; 75] = [
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1,  0, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1,
];
const Z_INV: [i8; 75] = [
    -1, 18, -1, 25, 26, 27, 30, 29,  7, 31, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, 24,  1, 12,  3,  8,  5,  6, 28, 21,  9, 10,
    -1, 11,  2, 16, 13, 14,  4, 22, 17, 19, -1, 20, 15,  0, 23,
];

pub fn decode(alphabet: Alphabet, data: &str) -> Option<Vec<u8>> {
    if !data.is_ascii() {
        return None;
    }
    let data = data.as_bytes();
    let alphabet = match alphabet {
        Alphabet::Crockford => CROCKFORD_INV, // supports both upper and lower case
        Alphabet::Rfc4648 { padding } => if padding { RFC4648_INV_PAD } else { RFC4648_INV }
        Alphabet::Rfc4648Lower { padding } => if padding { RFC4648_INV_LOWER_PAD } else { RFC4648_INV_LOWER }
        Alphabet::Rfc4648Hex { padding } => if padding { RFC4648_INV_HEX_PAD } else { RFC4648_INV_HEX }
        Alphabet::Rfc4648HexLower { padding } => if padding { RFC4648_INV_HEX_LOWER_PAD } else { RFC4648_INV_HEX_LOWER }
        Alphabet::Z => Z_INV,
    };
    let mut unpadded_data_length = data.len();
    for i in 1..min(6, data.len()) + 1 {
        if data[data.len() - i] != b'=' {
            break;
        }
        unpadded_data_length -= 1;
    }
    let output_length = unpadded_data_length * 5 / 8;
    let mut ret = Vec::with_capacity((output_length + 4) / 5 * 5);
    for chunk in data.chunks(8) {
        let buf = {
            let mut buf = [0u8; 8];
            for (i, &c) in chunk.iter().enumerate() {
                match alphabet.get(c.wrapping_sub(b'0') as usize) {
                    Some(&-1) | None => return None,
                    Some(&value) => buf[i] = value as u8,
                };
            }
            buf
        };
        ret.push((buf[0] << 3) | (buf[1] >> 2));
        ret.push((buf[1] << 6) | (buf[2] << 1) | (buf[3] >> 4));
        ret.push((buf[3] << 4) | (buf[4] >> 1));
        ret.push((buf[4] << 7) | (buf[5] << 2) | (buf[6] >> 3));
        ret.push((buf[6] << 5) | buf[7]);
    }
    ret.truncate(output_length);
    Some(ret)
}

#[cfg(test)]
#[allow(dead_code, unused_attributes)]
mod test {
    use super::Alphabet::{Crockford, Rfc4648, Rfc4648Hex, Rfc4648HexLower, Rfc4648Lower, Z};
    use super::{decode, encode};
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt::{Debug, Error, Formatter};
    use quickcheck::{Arbitrary, Gen};

    #[derive(Clone)]
    struct B32 {
        c: u8,
    }

    impl Arbitrary for B32 {
        fn arbitrary(g: &mut Gen) -> B32 {
            B32 {
                c: *g.choose(b"0123456789ABCDEFGHJKMNPQRSTVWXYZ").unwrap(),
            }
        }
    }

    impl Debug for B32 {
        fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
            (self.c as char).fmt(f)
        }
    }

    #[test]
    fn masks_crockford() {
        assert_eq!(
            encode(Crockford, &[0xF8, 0x3E, 0x0F, 0x83, 0xE0]),
            "Z0Z0Z0Z0"
        );
        assert_eq!(
            encode(Crockford, &[0x07, 0xC1, 0xF0, 0x7C, 0x1F]),
            "0Z0Z0Z0Z"
        );
        assert_eq!(
            decode(Crockford, "Z0Z0Z0Z0").unwrap(),
            [0xF8, 0x3E, 0x0F, 0x83, 0xE0]
        );
        assert_eq!(
            decode(Crockford, "0Z0Z0Z0Z").unwrap(),
            [0x07, 0xC1, 0xF0, 0x7C, 0x1F]
        );
    }

    #[test]
    fn masks_rfc4648() {
        assert_eq!(
            encode(Rfc4648 { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7A7H7A7H"
        );
        assert_eq!(
            encode(Rfc4648 { padding: false }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "O7A7O7A7"
        );
        assert_eq!(
            decode(Rfc4648 { padding: false }, "7A7H7A7H").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7]
        );
        assert_eq!(
            decode(Rfc4648 { padding: false }, "O7A7O7A7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F]
        );
        assert_eq!(
            encode(Rfc4648 { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "7A7H7AY"
        );
    }

    #[test]
    fn masks_rfc4648_pad() {
        assert_eq!(
            encode(Rfc4648 { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7A7H7A7H"
        );
        assert_eq!(
            encode(Rfc4648 { padding: true }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "O7A7O7A7"
        );
        assert_eq!(
            decode(Rfc4648 { padding: true }, "7A7H7A7H").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7]
        );
        assert_eq!(
            decode(Rfc4648 { padding: true }, "O7A7O7A7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F]
        );
        assert_eq!(
            encode(Rfc4648 { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "7A7H7AY="
        );
    }

    #[test]
    fn masks_rfc4648_lower() {
        assert_eq!(
            encode(Rfc4648Lower { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7a7h7a7h"
        );
        assert_eq!(
            encode(Rfc4648Lower { padding: false }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "o7a7o7a7"
        );
        assert_eq!(
            decode(Rfc4648Lower { padding: false }, "7a7h7a7h").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7]
        );
        assert_eq!(
            decode(Rfc4648Lower { padding: false }, "o7a7o7a7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F]
        );
        assert_eq!(
            encode(Rfc4648Lower { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "7a7h7ay"
        );
    }

    #[test]
    fn masks_rfc4648_lower_pad() {
        assert_eq!(
            encode(Rfc4648Lower { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "7a7h7a7h"
        );
        assert_eq!(
            encode(Rfc4648Lower { padding: true }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "o7a7o7a7"
        );
        assert_eq!(
            decode(Rfc4648Lower { padding: true }, "7a7h7a7h").unwrap(),
            [0xF8, 0x3E, 0x7F, 0x83, 0xE7]
        );
        assert_eq!(
            decode(Rfc4648Lower { padding: true }, "o7a7o7a7").unwrap(),
            [0x77, 0xC1, 0xF7, 0x7C, 0x1F]
        );
        assert_eq!(
            encode(Rfc4648Lower { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "7a7h7ay="
        );
    }

    #[test]
    fn masks_rfc4648_hex() {
        assert_eq!(
            encode(Rfc4648Hex { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "V0V7V0V7"
        );
        assert_eq!(
            encode(Rfc4648Hex { padding: false }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "EV0VEV0V"
        );
        assert_eq!(
            decode(Rfc4648Hex { padding: false }, "7A7H7A7H").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1]
        );
        assert_eq!(
            decode(Rfc4648Hex { padding: false }, "O7A7O7A7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47]
        );
        assert_eq!(
            encode(Rfc4648Hex { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "V0V7V0O"
        );
    }

    #[test]
    fn masks_rfc4648_hex_pad() {
        assert_eq!(
            encode(Rfc4648Hex { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "V0V7V0V7"
        );
        assert_eq!(
            encode(Rfc4648Hex { padding: true }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "EV0VEV0V"
        );
        assert_eq!(
            decode(Rfc4648Hex { padding: true }, "7A7H7A7H").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1]
        );
        assert_eq!(
            decode(Rfc4648Hex { padding: true }, "O7A7O7A7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47]
        );
        assert_eq!(
            encode(Rfc4648Hex { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "V0V7V0O="
        );
    }

    #[test]
    fn masks_rfc4648_hex_lower() {
        assert_eq!(
            encode(Rfc4648HexLower { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "v0v7v0v7"
        );
        assert_eq!(
            encode(Rfc4648HexLower { padding: false }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "ev0vev0v"
        );
        assert_eq!(
            decode(Rfc4648HexLower { padding: false }, "7a7h7a7h").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1]
        );
        assert_eq!(
            decode(Rfc4648HexLower { padding: false }, "o7a7o7a7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47]
        );
        assert_eq!(
            encode(Rfc4648HexLower { padding: false }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "v0v7v0o"
        );
    }

    #[test]
    fn masks_rfc4648_hex_lower_pad() {
        assert_eq!(
            encode(Rfc4648HexLower { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83, 0xE7]),
            "v0v7v0v7"
        );
        assert_eq!(
            encode(Rfc4648HexLower { padding: true }, &[0x77, 0xC1, 0xF7, 0x7C, 0x1F]),
            "ev0vev0v"
        );
        assert_eq!(
            decode(Rfc4648HexLower { padding: true }, "7a7h7a7h").unwrap(),
            [0x3A, 0x8F, 0x13, 0xA8, 0xF1]
        );
        assert_eq!(
            decode(Rfc4648HexLower { padding: true }, "o7a7o7a7").unwrap(),
            [0xC1, 0xD4, 0x7C, 0x1D, 0x47]
        );
        assert_eq!(
            encode(Rfc4648HexLower { padding: true }, &[0xF8, 0x3E, 0x7F, 0x83]),
            "v0v7v0o="
        );
    }

    #[test]
    fn masks_z() {
        assert_eq!(
            encode(Z, &[0xF8, 0x3E, 0x0F, 0x83, 0xE0]),
            "9y9y9y9y"
        );
        assert_eq!(
            encode(Z, &[0x07, 0xC1, 0xF0, 0x7C, 0x1F]),
            "y9y9y9y9"
        );
        assert_eq!(
            decode(Z, "9y9y9y9y").unwrap(),
            [0xF8, 0x3E, 0x0F, 0x83, 0xE0]
        );
        assert_eq!(
            decode(Z, "y9y9y9y9").unwrap(),
            [0x07, 0xC1, 0xF0, 0x7C, 0x1F]
        );
    }

    #[test]
    fn padding() {
        let num_padding = [0, 6, 4, 3, 1];
        for i in 1..6 {
            let encoded = encode(
                Rfc4648 { padding: true },
                (0..(i as u8)).collect::<Vec<u8>>().as_ref(),
            );
            assert_eq!(encoded.len(), 8);
            for j in 0..(num_padding[i % 5]) {
                assert_eq!(encoded.as_bytes()[encoded.len() - j - 1], b'=');
            }
            for j in 0..(8 - num_padding[i % 5]) {
                assert!(encoded.as_bytes()[j] != b'=');
            }
        }
    }

    #[test]
    fn invertible_crockford() {
        fn test(data: Vec<u8>) -> bool {
            decode(Crockford, encode(Crockford, data.as_ref()).as_ref()).unwrap() == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }

    #[test]
    fn invertible_rfc4648() {
        fn test(data: Vec<u8>) -> bool {
            decode(
                Rfc4648 { padding: true },
                encode(Rfc4648 { padding: true }, data.as_ref()).as_ref(),
            )
            .unwrap()
                == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }
    #[test]
    fn invertible_unpadded_rfc4648() {
        fn test(data: Vec<u8>) -> bool {
            decode(
                Rfc4648 { padding: false },
                encode(Rfc4648 { padding: false }, data.as_ref()).as_ref(),
            )
            .unwrap()
                == data
        }
        quickcheck::quickcheck(test as fn(Vec<u8>) -> bool)
    }

    #[test]
    fn lower_case() {
        fn test(data: Vec<B32>) -> bool {
            let data: String = data.iter().map(|e| e.c as char).collect();
            decode(Crockford, data.as_ref())
                == decode(Crockford, data.to_ascii_lowercase().as_ref())
        }
        quickcheck::quickcheck(test as fn(Vec<B32>) -> bool)
    }

    #[test]
    #[allow(non_snake_case)]
    fn iIlL1_oO0() {
        assert_eq!(decode(Crockford, "IiLlOo"), decode(Crockford, "111100"));
    }

    #[test]
    fn invalid_chars_crockford() {
        assert_eq!(decode(Crockford, ","), None)
    }

    #[test]
    fn invalid_chars_rfc4648() {
        assert_eq!(decode(Rfc4648 { padding: true }, ","), None)
    }

    #[test]
    fn invalid_chars_unpadded_rfc4648() {
        assert_eq!(decode(Rfc4648 { padding: false }, ","), None)
    }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct Readme;
