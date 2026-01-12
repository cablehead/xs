//! Zero-dependency compatible MIT licensed implementation [z-base-32](https://philzimmermann.com/docs/human-oriented-base-32-encoding.txt) encoding.

/// Alphabet used by zbase32
pub const ALPHABET: &[u8; 32] = b"ybndrfg8ejkmcpqxot1uwisza345h769";
const CHARACTER_CODE_TO_INDEX: [Option<u8>; 256] = {
    let mut char_code_to_index: [Option<u8>; 256] = [None; 256];
    let mut i = 0;
    let count = 32;

    loop {
        if i >= count {
            break;
        }

        let char_code = ALPHABET[i];
        char_code_to_index[char_code as usize] = Some(i as u8);
        i += 1;
    }
    char_code_to_index
};

/// Encode bytes using zbase32.
///
/// # Examples
///
/// ```
/// use z32;
///
/// let data = "The quick brown fox jumps over the lazy dog. 👀";
/// assert_eq!(z32::encode(data.as_bytes()),
///            "ktwgkedtqiwsg43ycj3g675qrbug66bypj4s4hdurbzzc3m1rb4go3jyptozw6jyctzsqmty6nx3dyy");
/// ```
pub fn encode(buf: &[u8]) -> String {
    let bits = buf.len() * 8;
    let capacity = if bits % 5 == 0 {
        bits / 5
    } else {
        bits / 5 + 1
    };

    let mut s = Vec::with_capacity(capacity);

    for p in (0..bits).step_by(5) {
        let i = p >> 3;
        let j = p & 7;
        if j <= 3 {
            s.push(ALPHABET[((buf[i] >> (3 - j)) & 0b11111) as usize]);
        } else {
            let of = j - 3;
            let h = (buf[i] << of) & 0b11111;
            let l = if i >= buf.len() - 1 {
                0
            } else {
                buf[i + 1] >> (8 - of)
            };
            s.push(ALPHABET[(h | l) as usize]);
        }
    }

    unsafe { String::from_utf8_unchecked(s) }
}

/// Decode zbase32 encoded string
///
/// This decodes full bytes. For instance, if you have `b"yy"`, you'll get one
/// byte back. `b"yy"` can enode 10 bits (2 * 5) which is truncated at the next
/// lower byte boundary.
///
/// # Examples
///
/// ```
/// use z32;
///
/// assert_eq!(z32::decode(b"qb1ze3m1").unwrap(), b"peter");
/// ```
pub fn decode(s: &[u8]) -> Result<Vec<u8>, Z32Error> {
    let mut position_bits = 0;
    let mut position_string = 0;
    let r = s.len() & 7;
    let q = (s.len() - r) / 8;

    let mut out: Vec<u8> = vec![0; (s.len() * 5 + 7) / 8];

    for _ in 0..q {
        let a = quintet(s, position_string)?;
        let b = quintet(s, position_string + 1)?;
        let c = quintet(s, position_string + 2)?;
        let d = quintet(s, position_string + 3)?;
        let e = quintet(s, position_string + 4)?;
        let f = quintet(s, position_string + 5)?;
        let g = quintet(s, position_string + 6)?;
        let h = quintet(s, position_string + 7)?;

        out[position_bits] = (a << 3) | (b >> 2);
        out[position_bits + 1] = ((b & 0b11) << 6) | (c << 1) | (d >> 4);
        out[position_bits + 2] = ((d & 0b1111) << 4) | (e >> 1);
        out[position_bits + 3] = ((e & 0b1) << 7) | (f << 2) | (g >> 3);
        out[position_bits + 4] = ((g & 0b111) << 5) | h;

        position_bits += 5;
        position_string += 8;
    }

    if r == 0 {
        return Ok(out[0..position_bits].to_vec());
    }

    let a = quintet(s, position_string)?;
    let b = quintet(s, position_string + 1)?;

    out[position_bits] = (a << 3) | (b >> 2);

    if r <= 2 {
        return Ok(out[0..position_bits + 1].to_vec());
    }

    let c = quintet(s, position_string + 2)?;
    let d = quintet(s, position_string + 3)?;
    out[position_bits + 1] = ((b & 0b11) << 6) | (c << 1) | (d >> 4);

    if r <= 4 {
        return Ok(out[0..position_bits + 2].to_vec());
    }

    let e = quintet(s, position_string + 4)?;
    out[position_bits + 2] = ((d & 0b1111) << 4) | (e >> 1);

    if r <= 5 {
        return Ok(out[0..position_bits + 3].to_vec());
    }

    let f = quintet(s, position_string + 5)?;
    let g = quintet(s, position_string + 6)?;
    out[position_bits + 3] = ((e & 0b1) << 7) | (f << 2) | (g >> 3);

    if r <= 7 {
        return Ok(out[0..position_bits + 4].to_vec());
    }

    let h = quintet(s, position_string + 7)?;
    out[position_bits + 4] = ((g & 0b111) << 5) | h;

    Ok(out[0..position_bits + 5].to_vec())
}

fn quintet(string: &[u8], position: usize) -> Result<u8, Z32Error> {
    if position >= string.len() {
        return Ok(0);
    };

    let c = string[position];

    match CHARACTER_CODE_TO_INDEX[c as usize] {
        Some(index) => Ok(index),
        None => Err(Z32Error::InvalidCharacter(c.into(), position)),
    }
}

#[derive(Debug, PartialEq)]
pub enum Z32Error {
    InvalidCharacter(char, usize),
}

impl std::error::Error for Z32Error {}

impl std::fmt::Display for Z32Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Z32Error::InvalidCharacter(char, index) => {
                write!(f, "Invalid z-base32 character {} at index {}", char, index)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::fill;

    #[test]
    fn basic() {
        let s = "The quick brown fox jumps over the lazy dog. 👀";
        let input = s.as_bytes();

        let encoded = encode(input);
        assert_eq!(
            encoded,
            "ktwgkedtqiwsg43ycj3g675qrbug66bypj4s4hdurbzzc3m1rb4go3jyptozw6jyctzsqmty6nx3dyy"
        );

        let decoded = decode(&encoded.as_bytes()).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn random() {
        let mut random_bytes: [u8; 32] = [0; 32];
        fill(&mut random_bytes);

        let encoded = encode(&random_bytes);
        assert_eq!(encoded.len(), 52);
    }

    #[test]
    fn public_key() {
        let s = "6ropkm1nz98qqwnotqz1tryk3mrfiw9u16iwzp1usci6kbqdfwho";

        let key: [u8; 32] = [
            241, 32, 213, 46, 66, 191, 206, 231, 80, 80, 139, 175, 40, 144, 10, 202, 200, 90, 211,
            243, 151, 171, 75, 182, 83, 179, 43, 229, 5, 195, 45, 57,
        ];

        assert_eq!(encode(&key), s);
        assert_eq!(decode(s.as_bytes()).unwrap(), key);
        assert_eq!(decode(encode(&key).as_bytes()).unwrap(), key);
    }

    const TEST_DATA: &[(&str, &[u8], &str)] = &[
        ("", &[], ""),
        ("y", &[0], "yy"),
        ("9", &[248], "9y"),
        ("com", &[100, 22], "comy"),
        ("yh", &[7], "yh"),
        ("6n9hq", &[240, 191, 199], "6n9hq"),
        ("4t7ye", &[212, 122, 4], "4t7ye"),
        (
            "yoearcwhngkq1s46",
            &[4, 17, 130, 50, 156, 17, 148, 233, 91, 94],
            "yoearcwhngkq1s46",
        ),
        (
            "ybndrfg8ejkmcpqxot1uwisza345h769",
            &[
                0, 68, 50, 20, 199, 66, 84, 182, 53, 207, 132, 101, 58, 86, 215, 198, 117, 190,
                119, 223,
            ],
            "ybndrfg8ejkmcpqxot1uwisza345h769",
        ),
    ];

    #[test]
    fn test_encode() {
        for &(_, bytes, encoded) in TEST_DATA {
            assert_eq!(encode(bytes), encoded);
        }
    }

    #[test]
    fn test_decode() {
        for &(zbase32, bytes, _) in TEST_DATA {
            assert_eq!(decode(zbase32.as_bytes()).unwrap(), bytes);
        }
    }

    #[test]
    fn test_bad_input() {
        let test_data = [
            ("!!!", 0),
            ("~~~", 0),
            ("l", 0),
            ("I1I1I1", 0),
            ("ybndrfg8ejkmcpqxot1uwisza345H769", 28),
            ("bnℕe", 2),
            ("uv", 1),
        ];

        for (input, index) in test_data {
            assert_eq!(
                decode(input.as_bytes()),
                Err(Z32Error::InvalidCharacter(
                    input.as_bytes()[index].into(),
                    index
                ))
            );
        }
    }
}
