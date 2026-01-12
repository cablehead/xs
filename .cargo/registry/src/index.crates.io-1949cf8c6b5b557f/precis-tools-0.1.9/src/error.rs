use std::fmt;
use std::path::{Path, PathBuf};

/// Represents any kind of error that can occur while parsing files
#[derive(Debug)]
pub struct Error {
    pub(crate) mesg: String,
    pub(crate) line: Option<u64>,
    pub(crate) path: Option<PathBuf>,
}

impl Error {
    /// Create a new parse error from the given message.
    pub(crate) fn parse(msg: String) -> Error {
        Error {
            mesg: msg,
            line: None,
            path: None,
        }
    }

    /// Return the specific kind of this error.
    pub fn mesg(&self) -> &str {
        self.mesg.as_str()
    }

    /// Return the line number at which this error occurred, if available.
    pub fn line(&self) -> Option<u64> {
        self.line
    }

    /// Return the file path associated with this error, if one exists.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref path) = self.path {
            if let Some(line) = self.line {
                write!(f, "{}, {}:{}: ", self.mesg, path.display(), line)
            } else {
                write!(f, "{}, {}", self.mesg, path.display())
            }
        } else if let Some(line) = self.line {
            write!(f, "{}, line {}", self.mesg, line)
        } else {
            write!(f, "{}", self.mesg)
        }
    }
}

impl From<ucd_parse::Error> for Error {
    fn from(error: ucd_parse::Error) -> Self {
        Error {
            mesg: format!("Parse error: {}", error),
            line: error.line(),
            path: None,
        }
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::parse(msg.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error {
            mesg: format!("IO Error: {}", error),
            line: None,
            path: None,
        }
    }
}
