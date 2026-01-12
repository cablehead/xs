use core::fmt;

use super::parsers::parse_visual_attribute;

/// An attribute of Select Graphic Rendition(SGR)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VisualAttribute {
    /// A bold.
    Bold,
    /// A faint.
    Faint,
    /// An italic.
    Italic,
    /// An underline.
    Underline,
    /// A slow blink.
    SlowBlink,
    /// A rapid blink.
    RapidBlink,
    /// Reverse video or invert.
    Inverse,
    /// Conceal or hide.
    Hide,
    /// Crossed-out, or strike.
    Crossedout,
    /// A font.
    ///
    /// A value is in range `10..=19`
    Font(u8),
    /// A fraktur (gothic)
    Fraktur,
    /// Doubly underlined; or: not bold.
    DoubleUnderline,
    /// A proportional spacing.
    ProportionalSpacing,
    /// A foreground color.
    FgColor(AnsiColor),
    /// A background color.
    BgColor(AnsiColor),
    /// An underground color.
    UndrColor(AnsiColor),
    /// A framed.
    Framed,
    /// An encircled.
    Encircled,
    /// An overlined.
    Overlined,
    /// Ideogram underline or right side line.
    IgrmUnderline,
    /// Ideogram double underline, or double line on the right side.
    IgrmDoubleUnderline,
    /// Ideogram overline or left side line.
    IgrmOverline,
    /// Ideogram double overline, or double line on the left side.
    IgrmdDoubleOverline,
    /// Ideogram stress marking.
    IgrmStressMarking,
    /// Superscript.
    Superscript,
    /// Subscript.
    Subscript,
    /// Bold.
    Reset(u8),
}

impl VisualAttribute {
    /// Parse a visual attribute.
    ///
    /// # Example
    ///
    /// ```
    /// use ansitok::VisualAttribute;
    ///
    /// assert_eq!(VisualAttribute::parse("1"), Some(VisualAttribute::Bold));
    /// ```
    pub fn parse<S>(text: S) -> Option<Self>
    where
        S: AsRef<str>,
    {
        parse_visual_attribute(text.as_ref())
            .ok()
            .map(|(_, attr)| attr)
    }
}

impl fmt::Display for VisualAttribute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\u{1b}")?;

        use VisualAttribute::*;
        match self {
            Bold => "1".fmt(f)?,
            Faint => "2".fmt(f)?,
            Italic => "3".fmt(f)?,
            Underline => "4".fmt(f)?,
            SlowBlink => "5".fmt(f)?,
            RapidBlink => "6".fmt(f)?,
            Inverse => "7".fmt(f)?,
            Hide => "8".fmt(f)?,
            Crossedout => "9".fmt(f)?,
            Font(n) => n.fmt(f)?,
            Fraktur => "20".fmt(f)?,
            DoubleUnderline => "21".fmt(f)?,
            ProportionalSpacing => "26".fmt(f)?,
            Framed => "51".fmt(f)?,
            Encircled => "52".fmt(f)?,
            Overlined => "53".fmt(f)?,
            IgrmUnderline => "60".fmt(f)?,
            IgrmDoubleUnderline => "61".fmt(f)?,
            IgrmOverline => "62".fmt(f)?,
            IgrmdDoubleOverline => "63".fmt(f)?,
            IgrmStressMarking => "64".fmt(f)?,
            Superscript => "73".fmt(f)?,
            Subscript => "74".fmt(f)?,
            Reset(n) => n.fmt(f)?,
            FgColor(color) => write_color(f, color, "38")?,
            BgColor(color) => write_color(f, color, "48")?,
            UndrColor(color) => write_color(f, color, "58")?,
        };

        write!(f, "m")?;

        Ok(())
    }
}

/// A Color representation in ANSI sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnsiColor {
    /// A color from [VisualAttribute].
    ///
    /// An example: `ESC[39;49m`.
    Bit4(u8),
    /// An index color.
    ///
    /// An example: `ESC[38:5:⟨n⟩m`.
    Bit8(u8),
    /// A 3 digit color.
    ///
    /// An example: `ESC[48;2;⟨r⟩;⟨g⟩;⟨b⟩m`.
    Bit24 {
        /// Red.
        r: u8,
        /// Green.
        g: u8,
        /// Blue.
        b: u8,
    },
}

fn write_color(f: &mut fmt::Formatter, color: &AnsiColor, prefix: &str) -> fmt::Result {
    match color {
        AnsiColor::Bit4(b) => write!(f, "{}", b),
        AnsiColor::Bit8(b) => write!(f, "{};5;{}", prefix, b),
        AnsiColor::Bit24 { r, g, b } => write!(f, "{};2;{};{};{}", prefix, r, g, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vis_attr_display() {
        use VisualAttribute::*;

        macro_rules! assert_vis_attr {
            ($val:expr, $expected:expr) => {
                assert_eq!(write_to_string($val), $expected);
            };
        }

        assert_vis_attr!(Bold, "\u{1b}1m");
        assert_vis_attr!(Faint, "\u{1b}2m");
        assert_vis_attr!(Italic, "\u{1b}3m");
        assert_vis_attr!(Underline, "\u{1b}4m");
        assert_vis_attr!(SlowBlink, "\u{1b}5m");
        assert_vis_attr!(RapidBlink, "\u{1b}6m");
        assert_vis_attr!(Inverse, "\u{1b}7m");
        assert_vis_attr!(Hide, "\u{1b}8m");
        assert_vis_attr!(Crossedout, "\u{1b}9m");
        assert_vis_attr!(Fraktur, "\u{1b}20m");
        assert_vis_attr!(DoubleUnderline, "\u{1b}21m");
        assert_vis_attr!(ProportionalSpacing, "\u{1b}26m");
        assert_vis_attr!(Framed, "\u{1b}51m");
        assert_vis_attr!(Encircled, "\u{1b}52m");
        assert_vis_attr!(Overlined, "\u{1b}53m");
        assert_vis_attr!(IgrmUnderline, "\u{1b}60m");
        assert_vis_attr!(IgrmDoubleUnderline, "\u{1b}61m");
        assert_vis_attr!(IgrmOverline, "\u{1b}62m");
        assert_vis_attr!(IgrmdDoubleOverline, "\u{1b}63m");
        assert_vis_attr!(IgrmStressMarking, "\u{1b}64m");
        assert_vis_attr!(Superscript, "\u{1b}73m");
        assert_vis_attr!(Subscript, "\u{1b}74m");

        macro_rules! assert_list {
            ($val:expr) => {
                for i in 0..u8::MAX {
                    assert_eq!(write_to_string($val(i)), format!("\u{1b}{}m", i));
                }
            };
        }

        assert_list!(Font);
        assert_list!(Reset);
    }

    #[ignore = "It's a slow function so run only when needed"]
    #[test]
    fn test_vis_attr_color_display() {
        use VisualAttribute::*;

        macro_rules! assert_color {
            ($val:expr, $prefix:expr) => {
                for i in 0..u8::MAX {
                    let val = $val(AnsiColor::Bit4(i));
                    let got = write_to_string(val);
                    assert_eq!(got, format!("\u{1b}{}m", i));
                }

                for i in 0..u8::MAX {
                    let val = $val(AnsiColor::Bit8(i));
                    let got = write_to_string(val);
                    assert_eq!(got, format!("\u{1b}{};5;{}m", $prefix, i));
                }

                for r in 0..u8::MAX {
                    for g in 0..u8::MAX {
                        for b in 0..u8::MAX {
                            let val = $val(AnsiColor::Bit24 { r, g, b });
                            let got = write_to_string(val);
                            assert_eq!(got, format!("\u{1b}{};2;{};{};{}m", $prefix, r, g, b));
                        }
                    }
                }
            };
        }

        assert_color!(FgColor, "38");
        assert_color!(BgColor, "48");
        assert_color!(UndrColor, "58");
    }

    fn write_to_string<D>(d: D) -> String
    where
        D: core::fmt::Display,
    {
        use core::fmt::Write;

        let mut buf = String::new();
        write!(&mut buf, "{}", d).expect("failed to write");
        buf
    }
}
