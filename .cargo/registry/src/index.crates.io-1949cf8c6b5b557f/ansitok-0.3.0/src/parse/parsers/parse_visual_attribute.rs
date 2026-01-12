use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit0, combinator::map, IResult,
};

use crate::{AnsiColor, VisualAttribute};

pub(crate) fn parse_visual_attribute(input: &str) -> IResult<&str, VisualAttribute> {
    peak_parser(input)
}

fn peak_parser(input: &str) -> IResult<&str, VisualAttribute> {
    use parsers::*;

    alt((
        alt((gm_fg_color, gm_bg_color, gm_undr_color)),
        alt((
            gm_bg_b_0, gm_bg_b_1, gm_bg_b_2, gm_bg_b_3, gm_bg_b_4, gm_bg_b_5, gm_bg_b_6, gm_bg_b_7,
        )),
        alt((
            gm_font_0, gm_font_1, gm_font_2, gm_font_3, gm_font_4, gm_font_5, gm_font_6, gm_font_7,
            gm_font_8,
        )),
        alt((
            gm_reset_0,
            gm_reset_22,
            gm_reset_23,
            gm_reset_24,
            gm_reset_25,
            gm_reset_27,
            gm_reset_28,
            gm_reset_29,
            gm_reset_39,
            gm_reset_49,
            gm_reset_50,
            gm_reset_54,
            gm_reset_55,
            gm_reset_59,
            gm_reset_65,
            gm_reset_75,
        )),
        alt((
            gm_underline_double,
            gm_proportional_spacing,
            gm_framed,
            gm_encircled,
            gm_overlined,
            gm_reset_font,
            gm_fraktur,
        )),
        alt((
            gm_igrm_underline,
            gm_igrm_underline_double,
            gm_igrm_overline,
            gm_igrm_overline_double,
            gm_igrm_stress_marking,
            gm_superscript,
            gm_subscript,
        )),
        alt((
            gm_fg_0, gm_fg_1, gm_fg_2, gm_fg_3, gm_fg_4, gm_fg_5, gm_fg_6, gm_fg_7,
        )),
        alt((
            gm_bg_0, gm_bg_1, gm_bg_2, gm_bg_3, gm_bg_4, gm_bg_5, gm_bg_6, gm_bg_7,
        )),
        alt((
            gm_fg_b_0, gm_fg_b_1, gm_fg_b_2, gm_fg_b_3, gm_fg_b_4, gm_fg_b_5, gm_fg_b_6, gm_fg_b_7,
        )),
        alt((
            gm_bold,
            gm_faint,
            gm_italic,
            gm_underline,
            gm_blink_slow,
            gm_blink_rapid,
            gm_inverse,
            gm_hide,
            gm_crossedout,
        )),
    ))(input)
}

mod parsers {
    use super::*;
    use VisualAttribute::*;

    macro_rules! gm_parse {
        ($sig:ident, $val:expr, $ret:expr) => {
            pub fn $sig(input: &str) -> IResult<&str, VisualAttribute> {
                let (input, _) = nom::bytes::complete::tag($val)(input)?;
                Ok((input, $ret))
            }
        };
    }

    macro_rules! gm_parse_color {
        ($sig:ident, $val:expr, $ret:expr) => {
            pub fn $sig(input: &str) -> IResult<&str, VisualAttribute> {
                let (input, _) = nom::bytes::complete::tag($val)(input)?;
                let (input, _) = nom::bytes::complete::tag(";")(input)?;
                let (input, color) = parse_bit_color(input)?;

                let result = $ret(color);

                Ok((input, result))
            }
        };
    }

    fn parse_bit_color(input: &str) -> IResult<&str, AnsiColor> {
        alt((
            map(parse_8_bit_color, AnsiColor::Bit8),
            map(parse_24_bit_color, |[r, g, b]| AnsiColor::Bit24 { r, g, b }),
        ))(input)
    }

    fn parse_8_bit_color(input: &str) -> IResult<&str, u8> {
        let (input, _) = tag("5")(input)?;
        let (input, _) = tag(";")(input)?;
        let (input, index) = opt_u8(input, 0)?;

        Ok((input, index))
    }

    fn parse_24_bit_color(input: &str) -> IResult<&str, [u8; 3]> {
        let (input, _) = tag("2")(input)?;
        let (input, _) = tag(";")(input)?;
        let (input, r) = opt_u8(input, 0)?;
        let (input, _) = tag(";")(input)?;
        let (input, g) = opt_u8(input, 0)?;
        let (input, _) = tag(";")(input)?;
        let (input, b) = opt_u8(input, 0)?;

        Ok((input, [r, g, b]))
    }

    fn opt_u8(input: &str, default: u8) -> IResult<&str, u8> {
        let (input, nums) = digit0(input)?;
        if nums.is_empty() {
            return Ok((input, default));
        }

        let num = u8_from_dec(nums).unwrap_or(default);
        Ok((input, num))
    }

    fn u8_from_dec(input: &str) -> Result<u8, core::num::ParseIntError> {
        input.parse::<u8>()
    }

    // gm_parse!(gm_reset_0, "", Reset(0));
    gm_parse!(gm_reset_0, "0", Reset(0));
    gm_parse!(gm_bold, "1", Bold);
    gm_parse!(gm_faint, "2", Faint);
    gm_parse!(gm_italic, "3", Italic);
    gm_parse!(gm_underline, "4", Underline);
    gm_parse!(gm_blink_slow, "5", SlowBlink);
    gm_parse!(gm_blink_rapid, "6", RapidBlink);
    gm_parse!(gm_inverse, "7", Inverse);
    gm_parse!(gm_hide, "8", Hide);
    gm_parse!(gm_crossedout, "9", Crossedout);
    gm_parse!(gm_reset_font, "10", Reset(10));
    gm_parse!(gm_font_0, "11", Font(11));
    gm_parse!(gm_font_1, "12", Font(12));
    gm_parse!(gm_font_2, "13", Font(13));
    gm_parse!(gm_font_3, "14", Font(14));
    gm_parse!(gm_font_4, "15", Font(15));
    gm_parse!(gm_font_5, "16", Font(16));
    gm_parse!(gm_font_6, "17", Font(17));
    gm_parse!(gm_font_7, "18", Font(18));
    gm_parse!(gm_font_8, "19", Font(19));
    gm_parse!(gm_fraktur, "20", Fraktur);
    gm_parse!(gm_underline_double, "21", DoubleUnderline);
    gm_parse!(gm_reset_22, "22", Reset(22));
    gm_parse!(gm_reset_23, "23", Reset(23));
    gm_parse!(gm_reset_24, "24", Reset(24));
    gm_parse!(gm_reset_25, "25", Reset(25));
    gm_parse!(gm_proportional_spacing, "26", ProportionalSpacing);
    gm_parse!(gm_reset_27, "27", Reset(27));
    gm_parse!(gm_reset_28, "28", Reset(28));
    gm_parse!(gm_reset_29, "29", Reset(29));
    gm_parse!(gm_fg_0, "30", FgColor(AnsiColor::Bit4(30)));
    gm_parse!(gm_fg_1, "31", FgColor(AnsiColor::Bit4(31)));
    gm_parse!(gm_fg_2, "32", FgColor(AnsiColor::Bit4(32)));
    gm_parse!(gm_fg_3, "33", FgColor(AnsiColor::Bit4(33)));
    gm_parse!(gm_fg_4, "34", FgColor(AnsiColor::Bit4(34)));
    gm_parse!(gm_fg_5, "35", FgColor(AnsiColor::Bit4(35)));
    gm_parse!(gm_fg_6, "36", FgColor(AnsiColor::Bit4(36)));
    gm_parse!(gm_fg_7, "37", FgColor(AnsiColor::Bit4(37)));
    gm_parse!(gm_reset_39, "39", Reset(39));
    gm_parse!(gm_bg_0, "40", BgColor(AnsiColor::Bit4(40)));
    gm_parse!(gm_bg_1, "41", BgColor(AnsiColor::Bit4(41)));
    gm_parse!(gm_bg_2, "42", BgColor(AnsiColor::Bit4(42)));
    gm_parse!(gm_bg_3, "43", BgColor(AnsiColor::Bit4(43)));
    gm_parse!(gm_bg_4, "44", BgColor(AnsiColor::Bit4(44)));
    gm_parse!(gm_bg_5, "45", BgColor(AnsiColor::Bit4(45)));
    gm_parse!(gm_bg_6, "46", BgColor(AnsiColor::Bit4(46)));
    gm_parse!(gm_bg_7, "47", BgColor(AnsiColor::Bit4(47)));
    gm_parse!(gm_reset_49, "49", Reset(49));
    gm_parse!(gm_reset_50, "50", Reset(50));
    gm_parse!(gm_framed, "51", Framed);
    gm_parse!(gm_encircled, "52", Encircled);
    gm_parse!(gm_overlined, "53", Overlined);
    gm_parse!(gm_reset_54, "54", Reset(54));
    gm_parse!(gm_reset_55, "55", Reset(55));
    gm_parse!(gm_reset_59, "59", Reset(59));
    gm_parse!(gm_igrm_underline, "60", IgrmUnderline);
    gm_parse!(gm_igrm_underline_double, "61", IgrmDoubleUnderline);
    gm_parse!(gm_igrm_overline, "62", IgrmOverline);
    gm_parse!(gm_igrm_overline_double, "63", IgrmdDoubleOverline);
    gm_parse!(gm_igrm_stress_marking, "64", IgrmStressMarking);
    gm_parse!(gm_reset_65, "65", Reset(65));
    gm_parse!(gm_superscript, "73", Superscript);
    gm_parse!(gm_subscript, "74", Subscript);
    gm_parse!(gm_reset_75, "75", Reset(75));

    gm_parse!(gm_fg_b_0, "90", FgColor(AnsiColor::Bit4(90)));
    gm_parse!(gm_fg_b_1, "91", FgColor(AnsiColor::Bit4(91)));
    gm_parse!(gm_fg_b_2, "92", FgColor(AnsiColor::Bit4(92)));
    gm_parse!(gm_fg_b_3, "93", FgColor(AnsiColor::Bit4(93)));
    gm_parse!(gm_fg_b_4, "94", FgColor(AnsiColor::Bit4(94)));
    gm_parse!(gm_fg_b_5, "95", FgColor(AnsiColor::Bit4(95)));
    gm_parse!(gm_fg_b_6, "96", FgColor(AnsiColor::Bit4(96)));
    gm_parse!(gm_fg_b_7, "97", FgColor(AnsiColor::Bit4(97)));

    gm_parse!(gm_bg_b_0, "100", BgColor(AnsiColor::Bit4(100)));
    gm_parse!(gm_bg_b_1, "101", BgColor(AnsiColor::Bit4(101)));
    gm_parse!(gm_bg_b_2, "102", BgColor(AnsiColor::Bit4(102)));
    gm_parse!(gm_bg_b_3, "103", BgColor(AnsiColor::Bit4(103)));
    gm_parse!(gm_bg_b_4, "104", BgColor(AnsiColor::Bit4(104)));
    gm_parse!(gm_bg_b_5, "105", BgColor(AnsiColor::Bit4(105)));
    gm_parse!(gm_bg_b_6, "106", BgColor(AnsiColor::Bit4(106)));
    gm_parse!(gm_bg_b_7, "107", BgColor(AnsiColor::Bit4(107)));

    gm_parse_color!(gm_fg_color, "38", FgColor);
    gm_parse_color!(gm_bg_color, "48", BgColor);
    gm_parse_color!(gm_undr_color, "58", UndrColor);
}
