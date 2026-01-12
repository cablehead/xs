use nom::{
    branch::alt,
    bytes::{complete::take_until, streaming::tag},
    combinator::opt,
    IResult,
};

use crate::parse::escape_sequence::EscapeCode;

use super::parse_util::{parse_u32_default, parse_u8};

pub(crate) fn parse_escape_sequence(input: &str) -> IResult<&str, EscapeCode<'_>> {
    let (input, _) = tag("\u{1b}")(input)?;
    if input.is_empty() {
        return Ok((input, EscapeCode::Escape));
    }

    parse::peak_parser(input)
}

mod parse {
    use super::*;
    use parsers::*;

    pub fn peak_parser(input: &str) -> IResult<&str, EscapeCode<'_>> {
        alt((
            alt((
                escape,
                cursor_pos,
                cursor_up,
                cursor_down,
                cursor_forward,
                cursor_backward,
                cursor_save,
                cursor_restore,
                erase_display,
                erase_line,
                set_mode,
            )),
            alt((
                reset_mode,
                hide_cursor,
                show_cursor,
                cursor_to_app,
                set_new_line_mode,
                set_col_132,
                set_smooth_scroll,
                set_reverse_video,
                set_origin_rel,
                set_auto_wrap,
                set_auto_repeat,
                set_interlacing,
                set_linefeed,
            )),
            alt((
                set_cursorkey,
                set_vt52,
                set_col80,
                set_jump_scroll,
                set_normal_video,
                set_origin_abs,
                reset_auto_wrap,
                reset_auto_repeat,
                reset_interlacing,
                set_top_and_bottom,
                set_alternate_keypad,
                set_numeric_keypad,
            )),
            alt((
                set_uk_g0,
                set_uk_g1,
                set_us_g0,
                set_us_g1,
                set_g0_special,
                set_g1_special,
                set_g0_alternate,
                set_g1_alternate,
                set_g0_graph,
                set_g1_graph,
                set_single_shift2,
                set_single_shift3,
                graphics_mode, // greedy so must be at the end
            )),
        ))(input)
    }

    #[rustfmt::skip]
    mod parsers {
        use super::*;

        macro_rules! tag_parser {
            ($sig:ident, $val:expr, $ret:expr) => {
                pub fn $sig(input: &str) -> IResult<&str, EscapeCode<'_>> {
                    let (input, _) = nom::bytes::streaming::tag($val)(input)?;
                    Ok((input, $ret))
                }
            };
        }
        
        macro_rules! int_parser_1_u32 {
            ($sig:ident, $prefix:expr, $suffix:expr, $obj:expr) => {
                pub fn $sig(input: &str) -> IResult<&str, EscapeCode<'_>> {
                    let (input, _) = tag($prefix)(input)?;
                    let (input, am) = parse_u32_default(input, 1)?;
                    let (input, _) = tag($suffix)(input)?;
        
                    Ok((input, $obj(am)))
                }
            };
        }

        macro_rules! int_parser_2_u32 {
            ($sig:ident, $prefix:expr, $suffix:expr, $obj:expr) => {
                pub fn $sig(input: &str) -> IResult<&str, EscapeCode<'_>> {
                    let (input, _) = $prefix(input)?;
                    let (input, x) = parse_u32_default(input, 1)?;
                    let (input, _) = opt(tag(";"))(input)?;
                    let (input, y) = parse_u32_default(input, 1)?;
                    let (input, _) = $suffix(input)?;
        
                    Ok((input, $obj(x, y)))
                }
            };
        }

        macro_rules! int_parser_1_u8 {
            ($sig:ident, $prefix:expr, $suffix:expr, $obj:expr) => {
                pub fn $sig(input: &str) -> IResult<&str, EscapeCode<'_>> {
                    let (input, _) = tag($prefix)(input)?;
                    let (input, am) = parse_u8(input)?;
                    let (input, _) = tag($suffix)(input)?;
        
                    Ok((input, $obj(am)))
                }
            };
        }

        int_parser_2_u32!(cursor_pos        , tag("["), alt((tag("H"), tag("f"))), EscapeCode::CursorPos);
        int_parser_2_u32!(set_top_and_bottom, tag("["), tag("r"),                  EscapeCode::SetTopAndBottom);

        int_parser_1_u32!(cursor_up,       "[", "A", EscapeCode::CursorUp);
        int_parser_1_u32!(cursor_down,     "[", "B", EscapeCode::CursorDown);
        int_parser_1_u32!(cursor_forward,  "[", "C", EscapeCode::CursorForward);
        int_parser_1_u32!(cursor_backward, "[", "D", EscapeCode::CursorBackward);

        int_parser_1_u8!(set_mode,   "[=", "h", EscapeCode::SetMode);
        int_parser_1_u8!(reset_mode, "[=", "l", EscapeCode::ResetMode);

        tag_parser!(escape,               "\u{1b}", EscapeCode::Escape);
        tag_parser!(cursor_save,          "[s",     EscapeCode::CursorSave);
        tag_parser!(cursor_restore,       "[u",     EscapeCode::CursorRestore);
        tag_parser!(erase_display,        "[2J",    EscapeCode::EraseDisplay);
        tag_parser!(erase_line,           "[K",     EscapeCode::EraseLine);
        tag_parser!(hide_cursor,          "[?25l",  EscapeCode::HideCursor);
        tag_parser!(show_cursor,          "[?25h",  EscapeCode::ShowCursor);
        tag_parser!(cursor_to_app,        "[?1h",   EscapeCode::CursorToApp);
        tag_parser!(set_new_line_mode,    "[20h",   EscapeCode::SetNewLineMode);
        tag_parser!(set_col_132,          "[?3h",   EscapeCode::SetCol132);
        tag_parser!(set_smooth_scroll,    "[?4h",   EscapeCode::SetSmoothScroll);
        tag_parser!(set_reverse_video,    "[?5h",   EscapeCode::SetReverseVideo);
        tag_parser!(set_origin_rel,       "[?6h",   EscapeCode::SetOriginRelative);
        tag_parser!(set_auto_wrap,        "[?7h",   EscapeCode::SetAutoWrap);
        tag_parser!(set_auto_repeat,      "[?8h",   EscapeCode::SetAutoRepeat);
        tag_parser!(set_interlacing,      "[?9h",   EscapeCode::SetInterlacing);
        tag_parser!(set_linefeed,         "[20l",   EscapeCode::SetLineFeedMode);
        tag_parser!(set_cursorkey,        "[?1l",   EscapeCode::SetCursorKeyToCursor);
        tag_parser!(set_vt52,             "[?2l",   EscapeCode::SetVT52);
        tag_parser!(set_col80,            "[?3l",   EscapeCode::SetCol80);
        tag_parser!(set_jump_scroll,      "[?4l",   EscapeCode::SetJumpScrolling);
        tag_parser!(set_normal_video,     "[?5l",   EscapeCode::SetNormalVideo);
        tag_parser!(set_origin_abs,       "[?6l",   EscapeCode::SetOriginAbsolute);
        tag_parser!(reset_auto_wrap,      "[?7l",   EscapeCode::ResetAutoWrap);
        tag_parser!(reset_auto_repeat,    "[?8l",   EscapeCode::ResetAutoRepeat);
        tag_parser!(reset_interlacing,    "[?9l",   EscapeCode::ResetInterlacing);
        tag_parser!(set_alternate_keypad, "=",      EscapeCode::SetAlternateKeypad);
        tag_parser!(set_numeric_keypad,   ">",      EscapeCode::SetNumericKeypad);
        tag_parser!(set_uk_g0,            "(A",     EscapeCode::SetUKG0);
        tag_parser!(set_uk_g1,            ")A",     EscapeCode::SetUKG1);
        tag_parser!(set_us_g0,            "(B",     EscapeCode::SetUSG0);
        tag_parser!(set_us_g1,            ")B",     EscapeCode::SetUSG1);
        tag_parser!(set_g0_special,       "(0",     EscapeCode::SetG0SpecialChars);
        tag_parser!(set_g1_special,       ")0",     EscapeCode::SetG1SpecialChars);
        tag_parser!(set_g0_alternate,     "(1",     EscapeCode::SetG0AlternateChar);
        tag_parser!(set_g1_alternate,     ")1",     EscapeCode::SetG1AlternateChar);
        tag_parser!(set_g0_graph,         "(2",     EscapeCode::SetG0AltAndSpecialGraph);
        tag_parser!(set_g1_graph,         ")2",     EscapeCode::SetG1AltAndSpecialGraph);
        tag_parser!(set_single_shift2,    "N",      EscapeCode::SetSingleShift2);
        tag_parser!(set_single_shift3,    "O",      EscapeCode::SetSingleShift3);

        pub fn graphics_mode(input: &str) -> IResult<&str, EscapeCode> {
            let (input, _) = tag("[")(input)?;
            let (input, mode) = take_until("m")(input)?;
            let (input, _) = tag("m")(input)?;

            Ok((input, EscapeCode::SelectGraphicRendition(mode)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_parse {
        ($name:ident, $string:expr) => {
            #[test]
            fn $name() {
                let (_, ret) = parse_escape_sequence($string).unwrap();

                use std::fmt::Write;
                let mut buff = String::new();
                write!(&mut buff, "{}", ret).unwrap();

                assert_eq!(buff, $string);

                let (_, ret2) = parse_escape_sequence(&buff).unwrap();
                assert_eq!(ret, ret2);
            }
        };
    }

    test_parse!(cursor_pos, "\u{1b}[10;5H");
    test_parse!(cursor_up, "\u{1b}[5A");
    test_parse!(cursor_down, "\u{1b}[5B");
    test_parse!(cursor_forward, "\u{1b}[5C");
    test_parse!(cursor_backward, "\u{1b}[5D");
    test_parse!(cursor_save, "\u{1b}[s");
    test_parse!(cursor_restore, "\u{1b}[u");

    test_parse!(erase_display, "\u{1b}[2J");
    test_parse!(erase_line, "\u{1b}[K");

    test_parse!(set_video_mode_a, "\u{1b}[4m");
    test_parse!(set_video_mode_b, "\u{1b}[4;42m");
    test_parse!(set_video_mode_c, "\u{1b}[4;31;42m");
    test_parse!(set_video_mode_d, "\u{1b}[4;31;42;42;42m");

    test_parse!(reset_mode, "\u{1b}[=13l");
    test_parse!(set_mode, "\u{1b}[=7h");

    test_parse!(show_cursor, "\u{1b}[?25h");
    test_parse!(hide_cursor, "\u{1b}[?25l");
    test_parse!(cursor_to_app, "\u{1b}[?1h");

    test_parse!(set_newline_mode, "\u{1b}[20h");
    test_parse!(set_column_132, "\u{1b}[?3h");
    test_parse!(set_smooth_scroll, "\u{1b}[?4h");
    test_parse!(set_reverse_video, "\u{1b}[?5h");
    test_parse!(set_origin_rel, "\u{1b}[?6h");
    test_parse!(set_auto_wrap, "\u{1b}[?7h");
    test_parse!(set_auto_repeat, "\u{1b}[?8h");
    test_parse!(set_interlacing, "\u{1b}[?9h");

    test_parse!(set_cursor_key_to_cursor, "\u{1b}[?1l");

    test_parse!(set_linefeed, "\u{1b}[20l");
    test_parse!(set_vt52, "\u{1b}[?2l");
    test_parse!(set_col80, "\u{1b}[?3l");
    test_parse!(set_jump_scroll, "\u{1b}[?4l");
    test_parse!(set_normal_video, "\u{1b}[?5l");
    test_parse!(set_origin_abs, "\u{1b}[?6l");
    test_parse!(reset_auto_wrap, "\u{1b}[?7l");
    test_parse!(reset_auto_repeat, "\u{1b}[?8l");
    test_parse!(reset_interlacing, "\u{1b}[?9l");

    test_parse!(set_alternate_keypad, "\u{1b}=");
    test_parse!(set_numeric_keypad, "\u{1b}>");
    test_parse!(set_uk_g0, "\u{1b}(A");
    test_parse!(set_uk_g1, "\u{1b})A");
    test_parse!(set_us_g0, "\u{1b}(B");
    test_parse!(set_us_g1, "\u{1b})B");
    test_parse!(set_g0_special, "\u{1b}(0");
    test_parse!(set_g1_special, "\u{1b})0");
    test_parse!(set_g0_alternate, "\u{1b}(1");
    test_parse!(set_g1_alternate, "\u{1b})1");
    test_parse!(set_g0_graph, "\u{1b}(2");
    test_parse!(set_g1_graph, "\u{1b})2");
    test_parse!(set_single_shift2, "\u{1b}N");
    test_parse!(set_single_shift3, "\u{1b}O");

    macro_rules! test_parse_default {
        ($name:ident, $string:expr) => {
            #[test]
            fn $name() {
                let mut buff = String::new();
                let (_, ret) = parse_escape_sequence($string).unwrap();

                use std::fmt::Write;
                write!(&mut buff, "{}", ret).unwrap();

                let ret2 = parse_escape_sequence(&buff);
                assert!(ret2.is_ok());

                let ret2 = ret2.unwrap().1;
                assert_eq!(ret, ret2);
            }
        };
    }

    test_parse_default!(cursor_pos_default, "\u{1b}[H");
    test_parse_default!(cursor_up_default, "\u{1b}[A");
}
