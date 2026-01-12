// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Parse a year from a string.
//!
//! The year must be parsed to a string first, this is to handle a specific GNU
//! compatibility quirk. According to the GNU documentation: "if the year is 68
//! or smaller, then 2000 is added to it; otherwise, if year is less than 100,
//! then 1900 is added to it." This adjustment only applies to two-digit year
//! strings. For example, `"00"` is interpreted as `2000`, whereas `"0"`,
//! `"000"`, or `"0000"` are interpreted as `0`.

use winnow::{stream::AsChar, token::take_while, ModalResult, Parser};

use super::primitive::s;

// TODO: Leverage `TryFrom` trait.
pub(super) fn year_from_str(year_str: &str) -> Result<u16, &'static str> {
    let mut year = year_str
        .parse::<u16>()
        .map_err(|_| "year must be a valid u16 number")?;

    // If year is 68 or smaller, then 2000 is added to it; otherwise, if year
    // is less than 100, then 1900 is added to it.
    //
    // GNU quirk: this only applies to two-digit years. For example,
    // "98-01-01" will be parsed as "1998-01-01", whereas "098-01-01" will be
    // parsed as "0098-01-01".
    if year_str.len() == 2 {
        if year <= 68 {
            year += 2000
        } else {
            year += 1900
        }
    }

    // 2147485547 is the maximum value accepted by GNU, but chrono only
    // behaves like GNU for years in the range: [0, 9999], so we keep in the
    // range [0, 9999].
    //
    // See discussion in https://github.com/uutils/parse_datetime/issues/160.
    if year > 9999 {
        return Err("year must be no greater than 9999");
    }

    Ok(year)
}

pub(super) fn year_str<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    s(take_while(1.., AsChar::is_dec_digit)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::year_from_str;

    #[test]
    fn test_year() {
        // 2-characters are converted to 19XX/20XX
        assert_eq!(year_from_str("10").unwrap(), 2010u16);
        assert_eq!(year_from_str("68").unwrap(), 2068u16);
        assert_eq!(year_from_str("69").unwrap(), 1969u16);
        assert_eq!(year_from_str("99").unwrap(), 1999u16);

        // 3,4-characters are converted verbatim
        assert_eq!(year_from_str("468").unwrap(), 468u16);
        assert_eq!(year_from_str("469").unwrap(), 469u16);
        assert_eq!(year_from_str("1568").unwrap(), 1568u16);
        assert_eq!(year_from_str("1569").unwrap(), 1569u16);

        // years greater than 9999 are not accepted
        assert!(year_from_str("10000").is_err());
    }
}
