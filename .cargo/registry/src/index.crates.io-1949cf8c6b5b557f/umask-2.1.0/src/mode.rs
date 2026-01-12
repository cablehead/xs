use std::{
    fmt::{self, Display, Formatter, Write},
    io,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not},
    path::Path,
};

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use thiserror::Error;

pub type Class = u32;

pub const EXTRA: Class = 0b111000000000;
pub const USER: Class = 0b111000000;
pub const GROUP: Class = 0b000111000;
pub const OTHERS: Class = 0b000000111;
pub const ALL: Class = 0b111111111;

pub type Permission = u32;

pub const READ: Permission = 0b100100100;
pub const WRITE: Permission = 0b010010010;
pub const EXEC: Permission = 0b001001001;

pub type ExtraPermission = u32;

/// When the sticky bit is set on a directory, files in that directory can only be
/// deleted by the owner.
pub const STICKY: ExtraPermission = 0o1000;
/// When the setgid bit is set on a an executable file, the file will be executed
/// by with the permissions of the file's group instead of the executing user's group.
///
/// When set on a directory, files and subdirectories created in it are assigned the
/// same group id as the parent directory.
pub const SETGID: ExtraPermission = 0o2000;
/// When the setuid bit is set on a an executable file, the file will be executed by
/// with the permissions of the file's owner instead of the executing user.
pub const SETUID: ExtraPermission = 0o4000;

pub const USER_READ: Mode = Mode::new().with_class_perm(USER, READ);
pub const USER_WRITE: Mode = Mode::new().with_class_perm(USER, WRITE);
pub const USER_EXEC: Mode = Mode::new().with_class_perm(USER, EXEC);
pub const GROUP_READ: Mode = Mode::new().with_class_perm(GROUP, READ);
pub const GROUP_WRITE: Mode = Mode::new().with_class_perm(GROUP, WRITE);
pub const GROUP_EXEC: Mode = Mode::new().with_class_perm(GROUP, EXEC);
pub const OTHERS_READ: Mode = Mode::new().with_class_perm(OTHERS, READ);
pub const OTHERS_WRITE: Mode = Mode::new().with_class_perm(OTHERS, WRITE);
pub const OTHERS_EXEC: Mode = Mode::new().with_class_perm(OTHERS, EXEC);
pub const ALL_READ: Mode = Mode::new().with_class_perm(ALL, READ);
pub const ALL_WRITE: Mode = Mode::new().with_class_perm(ALL, WRITE);
pub const ALL_EXEC: Mode = Mode::new().with_class_perm(ALL, EXEC);

#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Mode {
    value: u32,
}

impl fmt::Debug for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<u32> for Mode {
    fn from(value: u32) -> Self {
        Self { value }
    }
}

impl From<Mode> for u32 {
    fn from(mode: Mode) -> Self {
        mode.value
    }
}

impl From<&Mode> for u32 {
    fn from(mode: &Mode) -> Self {
        mode.value
    }
}

impl BitAnd for Mode {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self {
            value: self.value & other.value,
        }
    }
}

impl BitAndAssign for Mode {
    fn bitand_assign(&mut self, other: Self) {
        self.value &= other.value;
    }
}

impl BitOr for Mode {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self {
            value: self.value | other.value,
        }
    }
}

impl BitOrAssign for Mode {
    fn bitor_assign(&mut self, other: Self) {
        self.value |= other.value;
    }
}

impl Not for Mode {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self { value: !self.value }
    }
}

impl Display for Mode {
    /// Formats the Mode.
    ///
    /// If you want to prevent the extra permission bits from being displayed,
    /// use [`Mode::without_any_extra()`] to remove them before calling format.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_char(if self.has(USER_READ) { 'r' } else { '-' })?;
        f.write_char(if self.has(USER_WRITE) { 'w' } else { '-' })?;
        f.write_char(if self.has_extra(SETUID) && self.has(USER_EXEC) {
            's'
        } else if self.has_extra(SETUID) {
            'S'
        } else if self.has(USER_EXEC) {
            'x'
        } else {
            '-'
        })?;
        f.write_char(if self.has(GROUP_READ) { 'r' } else { '-' })?;
        f.write_char(if self.has(GROUP_WRITE) { 'w' } else { '-' })?;
        f.write_char(if self.has_extra(SETGID) && self.has(GROUP_EXEC) {
            's'
        } else if self.has_extra(SETGID) {
            'S'
        } else if self.has(GROUP_EXEC) {
            'x'
        } else {
            '-'
        })?;
        f.write_char(if self.has(OTHERS_READ) { 'r' } else { '-' })?;
        f.write_char(if self.has(OTHERS_WRITE) { 'w' } else { '-' })?;
        f.write_char(if self.has_extra(STICKY) && self.has(OTHERS_EXEC) {
            't'
        } else if self.has_extra(STICKY) {
            'T'
        } else if self.has(OTHERS_EXEC) {
            'x'
        } else {
            '-'
        })?;
        Ok(())
    }
}

/// Parsing error.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid character '{0}' at position {1}")]
    InvalidChar(char, usize),
    /// Invalid length.
    #[error("not enough input")]
    NotEnoughInput,
    /// Trailing characters.
    #[error("trailing characters")]
    TrailingCharacters,
}

/// Represents Permissions for a file.
///
/// # Formatting
///
/// String representations of Modes include the extra permission bits,
/// which modify how the executable permissions are displayed if set.
/// If you don't want to include this functionality, call [`without_any_extra()`](Mode::without_any_extra())
/// before converting the Mode into a string.
impl Mode {
    /// Build a mode with absolutely no permission
    #[inline(always)]
    pub const fn new() -> Self {
        Self { value: 0 }
    }
    /// Build a mode with all permissions given to everybody.
    /// This does not include [`ExtraPermission`] bits.
    #[inline(always)]
    pub const fn all() -> Self {
        Self { value: 0b111111111 }
    }
    /// Return the mode for the given path.
    /// On non unix platforms, return `Mode::all()`
    #[allow(unused_variables)]
    pub fn try_from(path: &Path) -> Result<Self, io::Error> {
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&path)?;
            Ok(Mode::from(metadata.mode()))
        }
        #[cfg(not(unix))]
        Ok(Self::all())
    }
    /// Try to parse a mode from a string.
    pub fn parse<T: AsRef<str>>(s: T) -> Result<Self, ParseError> {
        let mut result = Mode::new();
        let mut i = s.as_ref().chars().enumerate();

        #[inline]
        fn expect_single(
            i: &mut impl Iterator<Item = (usize, char)>,
            n: char,
            m: Mode,
        ) -> Result<Mode, ParseError> {
            match i.next() {
                Some((_, c)) if c == n => Ok(m),
                Some((_, '-')) => Ok(Mode::new()),
                Some((pos, c)) => Err(ParseError::InvalidChar(c, pos)),
                None => Err(ParseError::NotEnoughInput),
            }
        }

        result |= expect_single(&mut i, 'r', USER_READ)?;
        result |= expect_single(&mut i, 'w', USER_WRITE)?;
        match i.next() {
            Some((_, 's')) => result |= Mode::from(SETUID) | USER_EXEC,
            Some((_, 'S')) => result |= SETUID.into(),
            Some((_, 'x')) => result |= USER_EXEC,
            Some((_, '-')) => (),
            Some((pos, c)) => return Err(ParseError::InvalidChar(c, pos)),
            None => return Err(ParseError::NotEnoughInput),
        }
        result |= expect_single(&mut i, 'r', GROUP_READ)?;
        result |= expect_single(&mut i, 'w', GROUP_WRITE)?;
        match i.next() {
            Some((_, 's')) => result |= Mode::from(SETGID) | GROUP_EXEC,
            Some((_, 'S')) => result |= SETGID.into(),
            Some((_, 'x')) => result |= GROUP_EXEC,
            Some((_, '-')) => (),
            Some((pos, c)) => return Err(ParseError::InvalidChar(c, pos)),
            None => return Err(ParseError::NotEnoughInput),
        }
        result |= expect_single(&mut i, 'r', OTHERS_READ)?;
        result |= expect_single(&mut i, 'w', OTHERS_WRITE)?;
        match i.next() {
            Some((_, 't')) => result |= Mode::from(STICKY) | OTHERS_EXEC,
            Some((_, 'T')) => result |= STICKY.into(),
            Some((_, 'x')) => result |= OTHERS_EXEC,
            Some((_, '-')) => (),
            Some((pos, c)) => return Err(ParseError::InvalidChar(c, pos)),
            None => return Err(ParseError::NotEnoughInput),
        }

        if i.next().is_none() {
            Ok(result)
        } else {
            Err(ParseError::TrailingCharacters)
        }
    }
    /// Finds if the mode indicates an executable file
    #[inline(always)]
    pub const fn is_exe(self) -> bool {
        (self.value & 0o111) != 0
    }
    /// Indicates whether the passed class/permissions are all present in self
    #[inline(always)]
    pub const fn has(self, other: Self) -> bool {
        self.value & other.value == other.value
    }
    /// Indicates whether the passed extra permission is present in self
    #[inline(always)]
    pub const fn has_extra(self, other: ExtraPermission) -> bool {
        self.value & other == other
    }
    /// Return a new mode, with the extra permission set
    /// (does nothing if the extra permission is already set for the mode)
    #[inline(always)]
    pub const fn with_extra(self, perm: ExtraPermission) -> Self {
        Self {
            value: self.value | perm,
        }
    }
    /// Return a new mode, without the extra permission set
    /// (does nothing if the extra permission is not set for the mode)
    #[inline(always)]
    pub const fn without_extra(self, perm: ExtraPermission) -> Self {
        Self {
            value: self.value & !perm,
        }
    }
    /// Return a new mode, without any extra permission bits set
    /// (does nothing if no extra permissions are set for the mode)
    pub const fn without_any_extra(self) -> Self {
        Self {
            value: self.value & !EXTRA,
        }
    }
    /// Return a new mode, with the permission added for the class
    /// (does nothing if the permission is already given to that class)
    #[inline(always)]
    pub const fn with_class_perm(self, class: Class, perm: Permission) -> Self {
        Self {
            value: self.value | (class & perm),
        }
    }
    /// return a new mode, with the permission removed for the class
    /// (does nothing if the permission is already given to that class)
    #[inline(always)]
    pub const fn without_class_perm(self, class: Class, perm: Permission) -> Self {
        Self {
            value: self.value & !(class & perm),
        }
    }
    /// add the class/permissions of the other mode
    #[inline(always)]
    pub const fn with(self, other: Mode) -> Self {
        Self {
            value: self.value | other.value,
        }
    }
    /// remove the class/permissions of the other mode
    #[inline(always)]
    pub const fn without(self, other: Mode) -> Self {
        Self {
            value: self.value & !other.value,
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[test]
fn test_build() {
    let mut m = Mode::new()
        .with_class_perm(ALL, READ)
        .with_class_perm(USER, WRITE);
    assert_eq!("rw-r--r--", m.to_string());
    m |= ALL_EXEC;
    assert_eq!("rwxr-xr-x", m.to_string());
    m &= !USER_READ;
    assert_eq!("-wxr-xr-x", m.to_string());
}

#[test]
fn test_print() {
    assert_eq!("rw-r--r--", Mode::from(0b110100100).to_string());
    assert_eq!("rw-r--r--", Mode::from(0o644).to_string());
    let mu = Mode::from(0o600);
    let mo = Mode::from(0o004);
    assert_eq!("rw-------", mu.to_string());
    assert_eq!("------r--", mo.to_string());
    let muo = mu | mo;
    assert_eq!("rw----r--", muo.to_string());
}

#[test]
fn test_extra_permissions() {
    let mut m = Mode::all()
        .with_extra(STICKY)
        .with_extra(SETUID)
        .with_extra(SETGID);
    assert_eq!("rwsrwsrwt", m.to_string());
    m &= !Mode::from(EXEC);
    assert_eq!("rwSrwSrwT", m.to_string());
    let m = m.without_extra(STICKY);
    assert_eq!("rwSrwSrw-", m.to_string());
}

#[test]
fn test_try_from_str() -> Result<(), ParseError> {
    assert_eq!(Mode::parse("---------")?, Mode::from(0o000));
    assert_eq!(Mode::parse("rw-r--r--")?, Mode::from(0b110100100));
    assert_eq!(Mode::parse("rw-r--r--")?, Mode::from(0o644));
    assert_eq!(Mode::parse("rw-------")?, Mode::from(0o600));
    assert_eq!(Mode::parse("------r--")?, Mode::from(0o004));

    let expected = Mode::all()
        .with_extra(STICKY)
        .with_extra(SETUID)
        .with_extra(SETGID);
    assert_eq!(Mode::parse("rwsrwsrwt")?, expected);

    assert!(matches!(
        Mode::parse("rw-r--r---"),
        Err(ParseError::TrailingCharacters)
    ));
    assert!(matches!(Mode::parse(""), Err(ParseError::NotEnoughInput)));
    assert!(matches!(
        Mode::parse("rw-r--r-"),
        Err(ParseError::NotEnoughInput)
    ));

    assert!(
        matches!(Mode::parse("xw-r--r---"), Err(ParseError::InvalidChar(c, pos)) if c == 'x' && pos == 0)
    );
    Ok(())
}
