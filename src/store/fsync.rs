use std::fmt;
use std::str::FromStr;
use std::time::Duration;

/// When the store fsyncs its journal.
///
/// Every append commits to the fjall journal before it returns. A commit is
/// crash-consistent on its own: a process crash loses nothing. What an fsync
/// adds is durability against power loss or a kernel crash. Without it the
/// unsynced tail of the journal can be lost, and recovery yields a stable
/// prefix of the stream.
///
/// Parse one from a string with [`FromStr`]:
///
/// ```
/// use std::time::Duration;
/// use xs::Fsync;
///
/// assert_eq!("always".parse::<Fsync>(), Ok(Fsync::Always));
/// assert_eq!("interval:250".parse::<Fsync>(), Ok(Fsync::Interval(Duration::from_millis(250))));
/// assert_eq!("never".parse::<Fsync>(), Ok(Fsync::Never));
/// assert_eq!(Fsync::default(), Fsync::Interval(Duration::from_millis(1000)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fsync {
    /// Fsync after every append, before `append` returns. Slowest, and the
    /// only mode where a returned append survives power loss.
    Always,
    /// Fsync on a background tick every `Duration`, and only when something
    /// was written since the last tick. Power loss can lose up to one
    /// interval of appends. This is the default, at one second.
    Interval(Duration),
    /// Never fsync. The OS decides when the journal reaches disk.
    Never,
}

impl Default for Fsync {
    fn default() -> Self {
        Fsync::Interval(Duration::from_millis(1000))
    }
}

impl fmt::Display for Fsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fsync::Always => write!(f, "always"),
            Fsync::Interval(d) => write!(f, "interval:{}", d.as_millis()),
            Fsync::Never => write!(f, "never"),
        }
    }
}

impl FromStr for Fsync {
    type Err = String;

    /// Parses `always`, `interval:<ms>`, or `never`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always" => Ok(Fsync::Always),
            "never" => Ok(Fsync::Never),
            _ if s.starts_with("interval:") => {
                let ms = s["interval:".len()..]
                    .parse::<u64>()
                    .map_err(|_| "Invalid interval for 'interval' fsync policy".to_string())?;
                if ms == 0 {
                    return Err("Interval must be >= 1 ms for 'interval' fsync policy".to_string());
                }
                Ok(Fsync::Interval(Duration::from_millis(ms)))
            }
            _ => Err("Invalid fsync policy: expected always, interval:<ms>, or never".to_string()),
        }
    }
}
