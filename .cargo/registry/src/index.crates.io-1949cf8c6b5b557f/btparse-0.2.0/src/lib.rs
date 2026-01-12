//! An error utility library for deserializing `std::backtrace::Backtrace`'s
//! based on its `Debug` format.
#![doc(html_root_url = "https://docs.rs/btparse/0.1.1")]
#![allow(clippy::try_err)]
use std::fmt;

mod deser;

/// A deserialized Backtrace.
///
/// # Example
///
/// ```rust
/// #![feature(backtrace)]
///
/// let backtrace = std::backtrace::Backtrace::force_capture();
/// let backtrace = btparse::deserialize(&backtrace).unwrap();
/// for frame in &backtrace.frames {
///     println!("{:?}", frame);
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct Backtrace {
    pub frames: Vec<Frame>,
}

/// A backtrace frame.
#[derive(Debug, PartialEq)]
pub struct Frame {
    pub function: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// An error that prevented a backtrace from being deserialized.
#[derive(Debug)]
pub struct Error {
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Disabled,
    Unsupported,
    UnexpectedInput(String),
    InvalidInput { expected: String, found: String },
    LineParse(String, std::num::ParseIntError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.kind)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "backtrace capture disabled"),
            Self::Unsupported => write!(f, "backtrace capture unsupported on this platform"),
            Self::UnexpectedInput(input) => write!(f, "encountered unexpected input: {:?}", input),
            Self::InvalidInput { expected, found } => write!(
                f,
                "invalid input, expected: {:?}, found: {:?}",
                expected, found
            ),
            Self::LineParse(input, e) => {
                write!(f, "invalid line input for line number: {:?} ({:?})", input, e)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<Kind> for Error {
    fn from(kind: Kind) -> Self {
        Self { kind }
    }
}

/// Deserialize a backtrace based on its debug format and return a parsed
/// representation containing a vector of frames.
pub fn deserialize(bt: &std::backtrace::Backtrace) -> Result<Backtrace, Error> {
    let bt_str = format!("{:?}", bt);
    deserialize_str(&bt_str)
}

fn deserialize_str(bt: &str) -> Result<Backtrace, Error> {
    let mut frames = vec![];
    let mut bt = deser::header(bt)?;

    loop {
        let (bt_next, frame) = deser::frame(bt)?;
        bt = bt_next;
        frames.push(frame);

        let (bt_next, had_comma) = deser::trailing_comma(bt);
        bt = bt_next;

        if !had_comma {
            break;
        }
    }

    let bt = deser::close_bracket(bt)?;

    if !bt.is_empty() {
        Err(Kind::UnexpectedInput(bt.into()))?;
    }

    Ok(Backtrace { frames })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtrace_deserialize_enabled() -> eyre::Result<()> {
        let bt = std::backtrace::Backtrace::force_capture();
        let bt_parsed = super::deserialize(&bt)?;
        dbg!(bt_parsed);

        Ok(())
    }

    #[test]
    fn backtrace_deserialize_disabled() -> eyre::Result<()> {
        let bt = std::backtrace::Backtrace::capture();
        let bt_parsed = super::deserialize(&bt);
        match bt_parsed {
            Ok(_) => panic!("this should not parse"),
            Err(Error {
                kind: Kind::Disabled,
                ..
            }) => (),
            Err(e) => Err(e)?,
        }

        Ok(())
    }

    #[test]
    fn deserialize_simple() -> eyre::Result<()> {
        let backtrace = r#"Backtrace [{fn: "fn1", file: "fi"le1", line: 1}, {fn: "fn2", line: 2}, {fn: "fn3", file: "file3"}, {fn: "fn4"}]"#;
        let expected = Backtrace {
            frames: vec![
                Frame {
                    function: "fn1".into(),
                    file: Some("fi\"le1".into()),
                    line: Some(1),
                },
                Frame {
                    function: "fn2".into(),
                    file: None,
                    line: Some(2),
                },
                Frame {
                    function: "fn3".into(),
                    file: Some("file3".into()),
                    line: None,
                },
                Frame {
                    function: "fn4".into(),
                    file: None,
                    line: None,
                },
            ],
        };
        let bt_parsed = super::deserialize_str(backtrace)?;
        dbg!(&bt_parsed);

        assert_eq!(expected.frames, bt_parsed.frames);

        Ok(())
    }
}
