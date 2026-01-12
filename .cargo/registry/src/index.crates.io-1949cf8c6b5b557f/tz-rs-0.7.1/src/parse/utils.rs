//! Some useful functions.

use crate::error::parse::ParseDataError;

/// Cursor type alias
pub(super) type Cursor<'a> = &'a [u8];

/// Read exactly `count` bytes and reduce remaining data
pub(super) fn read_exact<'a>(cursor: &mut Cursor<'a>, count: usize) -> Result<&'a [u8], ParseDataError> {
    match cursor.split_at_checked(count) {
        Some((result, tail)) => {
            *cursor = tail;
            Ok(result)
        }
        None => Err(ParseDataError::UnexpectedEof),
    }
}

/// Read exactly `N` bytes into an array and reduce remaining data
pub(super) fn read_chunk_exact<'a, const N: usize>(cursor: &mut Cursor<'a>) -> Result<&'a [u8; N], ParseDataError> {
    match cursor.split_first_chunk::<N>() {
        Some((result, tail)) => {
            *cursor = tail;
            Ok(result)
        }
        None => Err(ParseDataError::UnexpectedEof),
    }
}

/// Read bytes and compare them to the provided tag
pub(super) fn read_tag(cursor: &mut Cursor<'_>, tag: &[u8]) -> Result<(), ParseDataError> {
    if read_exact(cursor, tag.len())? == tag { Ok(()) } else { Err(ParseDataError::InvalidData) }
}

/// Read bytes if the remaining data is prefixed by the provided tag
pub(super) fn read_optional_tag(cursor: &mut Cursor<'_>, tag: &[u8]) -> Result<bool, ParseDataError> {
    if cursor.starts_with(tag) {
        read_exact(cursor, tag.len())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Read bytes as long as the provided predicate is true
pub(super) fn read_while<'a, F: Fn(&u8) -> bool>(cursor: &mut Cursor<'a>, f: F) -> Result<&'a [u8], ParseDataError> {
    read_exact(cursor, cursor.iter().position(|x| !f(x)).unwrap_or(cursor.len()))
}

/// Read bytes until the provided predicate is true
pub(super) fn read_until<'a, F: Fn(&u8) -> bool>(cursor: &mut Cursor<'a>, f: F) -> Result<&'a [u8], ParseDataError> {
    read_exact(cursor, cursor.iter().position(f).unwrap_or(cursor.len()))
}
