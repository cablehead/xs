use ansitok::{parse_ansi, EscapeCode};
use EscapeCode::*;

macro_rules! test_parse_ansi {
    ($name:ident, $string:expr, $expected:expr) => {
        #[test]
        fn $name() {
            println!("{:?}", parse_ansi($string).collect::<Vec<_>>());

            let sequences: Vec<_> = parse_ansi($string)
                .flat_map(|e| EscapeCode::parse(&$string[e.start()..e.end()]))
                .collect();
            assert_eq!(sequences, $expected);
        }
    };
}

test_parse_ansi!(empty, "", []);
test_parse_ansi!(
    parse_escape,
    "\x1b\x1b\x1b\x1b\x1b",
    [Escape, Escape, Escape, Escape, Escape]
);
test_parse_ansi!(cur_pos_1, "\x1b[32;102H", [CursorPos(32, 102)]);
test_parse_ansi!(cur_pos_2, "\x1b[32;102f", [CursorPos(32, 102)]);
test_parse_ansi!(cur_pos_3, "\x1b[32;102;H", []);
test_parse_ansi!(cur_pos_4, "\x1b[32;102;f", []);
test_parse_ansi!(cur_pos_5, "\x1b[467434;3332H", [CursorPos(467434, 3332)]);
test_parse_ansi!(cur_pos_6, "\x1b[467434;3332f", [CursorPos(467434, 3332)]);
test_parse_ansi!(cur_pos_7, "\x1b[23;f", [CursorPos(23, 1)]);
test_parse_ansi!(cur_pos_8, "\x1b[;23f", [CursorPos(1, 23)]);
test_parse_ansi!(cur_pos_empty_1, "\x1b[f", [CursorPos(1, 1)]);
test_parse_ansi!(cur_pos_empty_2, "\x1b[H", [CursorPos(1, 1)]);
test_parse_ansi!(cur_pos_up, "\x1b[100A", [CursorUp(100)]);
test_parse_ansi!(cur_pos_up_big, "\x1b[123213A", [CursorUp(123213)]);
test_parse_ansi!(cur_pos_up_empty, "\x1b[A", [CursorUp(1)]);
test_parse_ansi!(cur_pos_down, "\x1b[100B", [CursorDown(100)]);
test_parse_ansi!(cur_pos_down_big, "\x1b[123213B", [CursorDown(123213)]);
test_parse_ansi!(cur_pos_down_empty, "\x1b[B", [CursorDown(1)]);
test_parse_ansi!(cur_pos_forward, "\x1b[100C", [CursorForward(100)]);
test_parse_ansi!(cur_pos_forward_1, "\x1b[123213C", [CursorForward(123213)]);
test_parse_ansi!(cur_pos_forward_empty, "\x1b[C", [CursorForward(1)]);
test_parse_ansi!(cur_pos_backward, "\x1b[100D", [CursorBackward(100)]);
test_parse_ansi!(cur_pos_backward_1, "\x1b[123213D", [CursorBackward(123213)]);
test_parse_ansi!(cur_pos_backward_empty, "\x1b[D", [CursorBackward(1)]);
test_parse_ansi!(set_mode, "\x1b[=23h", [SetMode(23)]);
test_parse_ansi!(set_mode_1, "\x1b[=h", []);
test_parse_ansi!(set_mode_2, "\x1b[=512h", []);
test_parse_ansi!(reset_mode, "\x1b[=23l", [ResetMode(23)]);
test_parse_ansi!(reset_mode_1, "\x1b[=l", []);
test_parse_ansi!(reset_mode_2, "\x1b[=512l", []);
test_parse_ansi!(set_top_bot, "\x1b[1;43r", [SetTopAndBottom(1, 43)]);
test_parse_ansi!(set_top_bot_1, "\x1b[;43r", [SetTopAndBottom(1, 43)]);
test_parse_ansi!(set_top_bot_2, "\x1b[1;43r", [SetTopAndBottom(1, 43)]);
test_parse_ansi!(set_top_bot_3, "\x1b[1;r", [SetTopAndBottom(1, 1)]);
test_parse_ansi!(set_top_bot_4, "\x1b[;1r", [SetTopAndBottom(1, 1)]);
test_parse_ansi!(set_top_bot_5, "\x1b[;r", [SetTopAndBottom(1, 1)]);
test_parse_ansi!(set_top_bot_6, "\x1b[500;500r", [SetTopAndBottom(500, 500)]);
test_parse_ansi!(cur_save, "\x1b[s", [CursorSave]);
test_parse_ansi!(cur_res, "\x1b[u", [CursorRestore]);
test_parse_ansi!(erase_dis, "\x1b[2J", [EraseDisplay]);
test_parse_ansi!(erase_line, "\x1b[K", [EraseLine]);
test_parse_ansi!(cur_hide, "\x1b[?25l", [HideCursor]);
test_parse_ansi!(cur_show, "\x1b[?25h", [ShowCursor]);
test_parse_ansi!(cur_to_app, "\x1b[?1h", [CursorToApp]);
test_parse_ansi!(set_n_line_mode, "\x1b[20h", [SetNewLineMode]);
test_parse_ansi!(set_col132, "\x1b[?3h", [SetCol132]);
test_parse_ansi!(set_smoot_scroll, "\x1b[?4h", [SetSmoothScroll]);
test_parse_ansi!(set_reverse_video, "\x1b[?5h", [SetReverseVideo]);
test_parse_ansi!(set_origin_relative, "\x1b[?6h", [SetOriginRelative]);
test_parse_ansi!(set_auto_wrap, "\x1b[?7h", [SetAutoWrap]);
test_parse_ansi!(set_auto_repeat, "\x1b[?8h", [SetAutoRepeat]);
test_parse_ansi!(set_interlacing, "\x1b[?9h", [SetInterlacing]);
test_parse_ansi!(set_line_feed_mode, "\x1b[20l", [SetLineFeedMode]);
test_parse_ansi!(set_cur_key_cur, "\x1b[?1l", [SetCursorKeyToCursor]);
test_parse_ansi!(set_vt52, "\x1b[?2l", [SetVT52]);
test_parse_ansi!(set_col80, "\x1b[?3l", [SetCol80]);
test_parse_ansi!(set_jump_scroll, "\x1b[?4l", [SetJumpScrolling]);
test_parse_ansi!(set_norm_video, "\x1b[?5l", [SetNormalVideo]);
test_parse_ansi!(set_origin_abs, "\x1b[?6l", [SetOriginAbsolute]);
test_parse_ansi!(reset_autowrap, "\x1b[?7l", [ResetAutoWrap]);
test_parse_ansi!(reset_autorepeat, "\x1b[?8l", [ResetAutoRepeat]);
test_parse_ansi!(reset_interlacing, "\x1b[?9l", [ResetInterlacing]);
test_parse_ansi!(set_alt_keypad, "\x1b=", [SetAlternateKeypad]);
test_parse_ansi!(set_num_keypad, "\x1b>", [SetNumericKeypad]);
test_parse_ansi!(set_ukg0, "\x1b(A", [SetUKG0]);
test_parse_ansi!(set_ukg1, "\x1b)A", [SetUKG1]);
test_parse_ansi!(set_usg0, "\x1b(B", [SetUSG0]);
test_parse_ansi!(set_usg1, "\x1b)B", [SetUSG1]);
test_parse_ansi!(set_g0_spec_chars, "\x1b(0", [SetG0SpecialChars]);
test_parse_ansi!(set_g1_spec_chars, "\x1b)0", [SetG1SpecialChars]);
test_parse_ansi!(set_g0_alt_chars, "\x1b(1", [SetG0AlternateChar]);
test_parse_ansi!(set_g1_alt_chars, "\x1b)1", [SetG1AlternateChar]);
test_parse_ansi!(set_g0_spec_alt_chars, "\x1b(2", [SetG0AltAndSpecialGraph]);
test_parse_ansi!(set_g1_spec_alt_chars, "\x1b)2", [SetG1AltAndSpecialGraph]);
test_parse_ansi!(set_single_shft2, "\x1bN", [SetSingleShift2]);
test_parse_ansi!(set_single_shft3, "\x1bO", [SetSingleShift3]);

test_parse_ansi!(
    parse_0,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36m\x1b[1m-`",
    [
        ResetMode(25),
        ResetMode(7),
        SelectGraphicRendition("0"),
        SelectGraphicRendition("36"),
        SelectGraphicRendition("1"),
    ]
);
test_parse_ansi!(
    parse_1,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36;1;15;2m\x1b[1m-`",
    [
        ResetMode(25),
        ResetMode(7),
        SelectGraphicRendition("0"),
        SelectGraphicRendition("36;1;15;2"),
        SelectGraphicRendition("1"),
    ]
);
test_parse_ansi!(
    parse_2,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36;1;15;2m\x1b[1m-`",
    [
        ResetMode(25),
        ResetMode(7),
        SelectGraphicRendition("0"),
        SelectGraphicRendition("36;1;15;2"),
        SelectGraphicRendition("1"),
    ]
);
test_parse_ansi!(
    parse_3,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36;1;15;2;36;1;15;2m\x1b[1m-`",
    [
        ResetMode(25),
        ResetMode(7),
        SelectGraphicRendition("0"),
        SelectGraphicRendition("36;1;15;2;36;1;15;2"),
        SelectGraphicRendition("1"),
    ]
);
test_parse_ansi!(
    parse_4,
    "\x1b[H\x1b[123456H\x1b[;123456H\x1b[7asd;1234H\x1b[a;sd7H",
    [CursorPos(1, 1), CursorPos(123456, 1), CursorPos(1, 123456),]
);
test_parse_ansi!(
    parse_5,
    "\x1b\x1b[33mFoobar",
    [Escape, SelectGraphicRendition("33")]
);
test_parse_ansi!(
    parse_6,
    "\x1b[38;5;45mFoobar\x1b[0m",
    [
        SelectGraphicRendition("38;5;45"),
        SelectGraphicRendition("0")
    ]
);
