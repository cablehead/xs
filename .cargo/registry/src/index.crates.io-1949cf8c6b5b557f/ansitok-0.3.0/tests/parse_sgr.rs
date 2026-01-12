use ansitok::{
    parse_ansi_sgr,
    AnsiColor::*,
    Output,
    Output::Escape as esc,
    VisualAttribute::{self, *},
};

macro_rules! test_parse_sgr {
    ($name:ident, $string:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let sequences: Vec<_> = parse_ansi_sgr($string).collect();
            assert_eq!(sequences, $expected);
        }
    };
}

test_parse_sgr!(parse_empty, "", []); // is this correct? or we shall consider it a RESET 0?
test_parse_sgr!(
    parse_valid_ansi_sequence,
    "\x1b[38;5;45mFoobar\x1b[0m",
    [text("38;5;45mFoobar\x1b[0"),]
);
test_parse_sgr!(
    parse_invalid_ansi_sequence,
    "\x1b[38;5;45Foobar\x1b[0",
    [text("\x1b[38;5;45Foobar\x1b[0"),]
);
test_parse_sgr!(
    parse_invalid_ansi_sequence2,
    "38;5;45\x1b[0",
    [text("38;5;45\x1b[0"),]
);
test_parse_sgr!(parse_1, "38;5;45", [esc(FgColor(Bit8(45)))]);
test_parse_sgr!(parse_2, "5;45", [esc(SlowBlink), esc(BgColor(Bit4(45)))]);
test_parse_sgr!(
    parse_3,
    "48;2;127;0;255",
    [esc(BgColor(Bit24 {
        r: 127,
        g: 0,
        b: 255
    }))]
);
test_parse_sgr!(
    parse_4,
    "2;48;2;127;0;255",
    [
        esc(Faint),
        esc(BgColor(Bit24 {
            r: 127,
            g: 0,
            b: 255
        }))
    ]
);
test_parse_sgr!(
    parse_5,
    "1;2;3;38;2;255;255;0;0",
    [
        esc(Bold),
        esc(Faint),
        esc(Italic),
        esc(FgColor(Bit24 {
            r: 255,
            g: 255,
            b: 0,
        })),
        esc(Reset(0)),
    ]
);
test_parse_sgr!(parse_fg_8bit, "38;5;128", [esc(FgColor(Bit8(128)))]);
test_parse_sgr!(
    parse_fg_24bit,
    "38;2;1;2;3",
    [esc(FgColor(Bit24 { r: 1, g: 2, b: 3 }))]
);
test_parse_sgr!(
    test_some_sequence_1,
    "3;4;48;2;4;5;6;38;2;1;2;3",
    [
        esc(Italic),
        esc(Underline),
        esc(BgColor(Bit24 { r: 4, g: 5, b: 6 })),
        esc(FgColor(Bit24 { r: 1, g: 2, b: 3 }))
    ]
);
test_parse_sgr!(
    test_some_sequence_2,
    "3;4;44;31",
    [
        esc(Italic),
        esc(Underline),
        esc(BgColor(Bit4(44))),
        esc(FgColor(Bit4(31)))
    ]
);

#[test]
fn test_parsing_single_byte() {
    for i in 38..39 {
        let expected = expect_byte(i);

        let s = i.to_string();
        let got: Vec<_> = parse_ansi_sgr(&s).collect();

        assert_eq!(got, [expected]);
    }
}

fn expect_byte(b: u8) -> Output<'static, VisualAttribute> {
    match b {
        0 => esc(Reset(0)),
        1 => esc(Bold),
        2 => esc(Faint),
        3 => esc(Italic),
        4 => esc(Underline),
        5 => esc(SlowBlink),
        6 => esc(RapidBlink),
        7 => esc(Inverse),
        8 => esc(Hide),
        9 => esc(Crossedout),
        10 => esc(Reset(10)),
        n @ 11..=19 => esc(Font(n)),
        20 => esc(Fraktur),
        21 => esc(DoubleUnderline),
        n @ 22..=25 => esc(Reset(n)),
        26 => esc(ProportionalSpacing),
        n @ 27..=29 => esc(Reset(n)),
        n @ 30..=37 => esc(FgColor(Bit4(n))),
        38 => text("38"),
        39 => esc(Reset(39)),
        n @ 40..=47 => esc(BgColor(Bit4(n))),
        48 => text("48"),
        49 => esc(Reset(49)),
        50 => esc(Reset(50)),
        51 => esc(Framed),
        52 => esc(Encircled),
        53 => esc(Overlined),
        n @ 54..=55 => esc(Reset(n)),
        58 => text("58"),
        59 => esc(Reset(59)),
        60 => esc(IgrmUnderline),
        61 => esc(IgrmDoubleUnderline),
        62 => esc(IgrmOverline),
        63 => esc(IgrmdDoubleOverline),
        64 => esc(IgrmStressMarking),
        65 => esc(Reset(65)),
        73 => esc(Superscript),
        74 => esc(Subscript),
        75 => esc(Reset(75)),
        n @ 90..=97 => esc(FgColor(Bit4(n))),
        n @ 100..=107 => esc(BgColor(Bit4(n))),
        n => text(n.to_string()),
    }
}

fn text<'a, T>(s: impl Into<std::borrow::Cow<'a, str>>) -> Output<'a, T> {
    match s.into() {
        std::borrow::Cow::Owned(s) => {
            let s = s.into_boxed_str();
            let s = Box::leak(s);
            Output::Text(s)
        }
        std::borrow::Cow::Borrowed(s) => Output::Text(s),
    }
}
