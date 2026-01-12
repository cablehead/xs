use ansitok::{parse_ansi, Element};

macro_rules! test_parse_ansi {
    ($name:ident, $string:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let sequences: Vec<_> = parse_ansi($string).collect();
            assert_eq!(sequences, $expected);
        }
    };
}

fn csi(start: usize, end: usize) -> Element {
    Element::csi(start, end)
}

fn esc(start: usize, end: usize) -> Element {
    Element::esc(start, end)
}

fn sgr(start: usize, end: usize) -> Element {
    Element::sgr(start, end)
}

fn text(start: usize, end: usize) -> Element {
    Element::text(start, end)
}

test_parse_ansi!(empty, "", []);
test_parse_ansi!(
    parse_escape,
    "\x1b\x1b\x1b\x1b\x1b",
    [esc(0, 1), esc(1, 2), esc(2, 3), esc(3, 4), esc(4, 5)]
);
test_parse_ansi!(cur_pos_1, "\x1b[32;102H", [csi(0, 9)]);
test_parse_ansi!(cur_pos_2, "\x1b[32;102f", [csi(0, 9)]);
test_parse_ansi!(cur_pos_3, "\x1b[32;102;H", [csi(0, 10)]);
test_parse_ansi!(cur_pos_4, "\x1b[32;102;f", [csi(0, 10)]);
test_parse_ansi!(cur_pos_5, "\x1b[467434;3332H", [csi(0, 14)]);
test_parse_ansi!(cur_pos_6, "\x1b[467434;3332f", [csi(0, 14)]);
test_parse_ansi!(cur_pos_7, "\x1b[23;f", [csi(0, 6)]);
test_parse_ansi!(cur_pos_8, "\x1b[;23f", [csi(0, 6)]);
test_parse_ansi!(cur_pos_empty_1, "\x1b[f", [csi(0, 3)]);
test_parse_ansi!(cur_pos_empty_2, "\x1b[H", [csi(0, 3)]);
test_parse_ansi!(cur_pos_up, "\x1b[100A", [csi(0, 6)]);
test_parse_ansi!(cur_pos_up_big, "\x1b[123213A", [csi(0, 9)]);
test_parse_ansi!(cur_pos_up_empty, "\x1b[A", [csi(0, 3)]);
test_parse_ansi!(cur_pos_down, "\x1b[100B", [csi(0, 6)]);
test_parse_ansi!(cur_pos_down_big, "\x1b[123213B", [csi(0, 9)]);
test_parse_ansi!(cur_pos_down_empty, "\x1b[B", [csi(0, 3)]);
test_parse_ansi!(cur_pos_forward, "\x1b[100C", [csi(0, 6)]);
test_parse_ansi!(cur_pos_forward_1, "\x1b[123213C", [csi(0, 9)]);
test_parse_ansi!(cur_pos_forward_empty, "\x1b[C", [csi(0, 3)]);
test_parse_ansi!(cur_pos_backward, "\x1b[100D", [csi(0, 6)]);
test_parse_ansi!(cur_pos_backward_1, "\x1b[123213D", [csi(0, 9)]);
test_parse_ansi!(cur_pos_backward_empty, "\x1b[D", [csi(0, 3)]);
test_parse_ansi!(set_mode, "\x1b[=23h", [csi(0, 6)]);
test_parse_ansi!(set_mode_1, "\x1b[=h", [csi(0, 4)]);
test_parse_ansi!(set_mode_2, "\x1b[=512h", [csi(0, 7)]);
test_parse_ansi!(reset_mode, "\x1b[=23l", [csi(0, 6)]);
test_parse_ansi!(reset_mode_1, "\x1b[=l", [csi(0, 4)]);
test_parse_ansi!(reset_mode_2, "\x1b[=512l", [csi(0, 7)]);
test_parse_ansi!(set_top_bot, "\x1b[1;43r", [csi(0, 7)]);
test_parse_ansi!(set_top_bot_1, "\x1b[;43r", [csi(0, 6)]);
test_parse_ansi!(set_top_bot_2, "\x1b[1;43r", [csi(0, 7)]);
test_parse_ansi!(set_top_bot_3, "\x1b[1;r", [csi(0, 5)]);
test_parse_ansi!(set_top_bot_4, "\x1b[;1r", [csi(0, 5)]);
test_parse_ansi!(set_top_bot_5, "\x1b[;r", [csi(0, 4)]);
test_parse_ansi!(set_top_bot_6, "\x1b[500;500r", [csi(0, 10)]);
test_parse_ansi!(cur_save, "\x1b[s", [csi(0, 3)]);
test_parse_ansi!(cur_res, "\x1b[u", [csi(0, 3)]);
test_parse_ansi!(erase_dis, "\x1b[2J", [csi(0, 4)]);
test_parse_ansi!(erase_line, "\x1b[K", [csi(0, 3)]);
test_parse_ansi!(cur_hide, "\x1b[?25l", [csi(0, 6)]);
test_parse_ansi!(cur_show, "\x1b[?25h", [csi(0, 6)]);
test_parse_ansi!(cur_to_app, "\x1b[?1h", [csi(0, 5)]);
test_parse_ansi!(set_n_line_mode, "\x1b[20h", [csi(0, 5)]);
test_parse_ansi!(set_col132, "\x1b[?3h", [csi(0, 5)]);
test_parse_ansi!(set_smoot_scroll, "\x1b[?4h", [csi(0, 5)]);
test_parse_ansi!(set_reverse_video, "\x1b[?5h", [csi(0, 5)]);
test_parse_ansi!(set_origin_relative, "\x1b[?6h", [csi(0, 5)]);
test_parse_ansi!(set_auto_wrap, "\x1b[?7h", [csi(0, 5)]);
test_parse_ansi!(set_auto_repeat, "\x1b[?8h", [csi(0, 5)]);
test_parse_ansi!(set_interlacing, "\x1b[?9h", [csi(0, 5)]);
test_parse_ansi!(set_line_feed_mode, "\x1b[20l", [csi(0, 5)]);
test_parse_ansi!(set_cur_key_cur, "\x1b[?1l", [csi(0, 5)]);
test_parse_ansi!(set_vt52, "\x1b[?2l", [csi(0, 5)]);
test_parse_ansi!(set_col80, "\x1b[?3l", [csi(0, 5)]);
test_parse_ansi!(set_jump_scroll, "\x1b[?4l", [csi(0, 5)]);
test_parse_ansi!(set_norm_video, "\x1b[?5l", [csi(0, 5)]);
test_parse_ansi!(set_origin_abs, "\x1b[?6l", [csi(0, 5)]);
test_parse_ansi!(reset_autowrap, "\x1b[?7l", [csi(0, 5)]);
test_parse_ansi!(reset_autorepeat, "\x1b[?8l", [csi(0, 5)]);
test_parse_ansi!(reset_interlacing, "\x1b[?9l", [csi(0, 5)]);
test_parse_ansi!(set_alt_keypad, "\x1b=", [esc(0, 2)]);
test_parse_ansi!(set_num_keypad, "\x1b>", [esc(0, 2)]);
test_parse_ansi!(set_ukg0, "\x1b(A", [esc(0, 3)]);
test_parse_ansi!(set_ukg1, "\x1b)A", [esc(0, 3)]);
test_parse_ansi!(set_usg0, "\x1b(B", [esc(0, 3)]);
test_parse_ansi!(set_usg1, "\x1b)B", [esc(0, 3)]);
test_parse_ansi!(set_g0_spec_chars, "\x1b(0", [esc(0, 3)]);
test_parse_ansi!(set_g1_spec_chars, "\x1b)0", [esc(0, 3)]);
test_parse_ansi!(set_g0_alt_chars, "\x1b(1", [esc(0, 3)]);
test_parse_ansi!(set_g1_alt_chars, "\x1b)1", [esc(0, 3)]);
test_parse_ansi!(set_g0_spec_alt_chars, "\x1b(2", [esc(0, 3)]);
test_parse_ansi!(set_g1_spec_alt_chars, "\x1b)2", [esc(0, 3)]);
test_parse_ansi!(set_single_shft2, "\x1bN", [esc(0, 2)]);
test_parse_ansi!(set_single_shft3, "\x1bO", [esc(0, 2)]);

test_parse_ansi!(
    parse_0,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36m\x1b[1m-`",
    [
        csi(0, 6),
        csi(6, 11),
        sgr(11, 15),
        sgr(15, 20),
        sgr(20, 24),
        text(24, 26)
    ]
);
test_parse_ansi!(
    parse_1,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36;1;15;2m\x1b[1m-`",
    [
        csi(0, 6),
        csi(6, 11),
        sgr(11, 15),
        sgr(15, 27),
        sgr(27, 31),
        text(31, 33)
    ]
);
test_parse_ansi!(
    parse_2,
    "\x1b[=25l\x1b[=7l\x1b[0m\x1b[36;1;15;2;36;1;15;2m\x1b[1m-`",
    [
        csi(0, 6),
        csi(6, 11),
        sgr(11, 15),
        sgr(15, 37),
        sgr(37, 41),
        text(41, 43)
    ]
);
test_parse_ansi!(
    parse_4,
    "\x1b[H\x1b[123456H\x1b[;123456H\x1b[7asd;1234H\x1b[a;sd7H",
    [
        csi(0, 3),
        csi(3, 12),
        csi(12, 22),
        csi(22, 26),
        text(26, 34),
        csi(34, 37),
        text(37, 42),
    ]
);
test_parse_ansi!(
    parse_5,
    "\x1b\x1b[33mFoobar",
    [esc(0, 1), sgr(1, 6), text(6, 12),]
);
test_parse_ansi!(
    parse_6,
    "\x1b[38;5;45mFoobar\x1b[0m",
    [sgr(0, 10), text(10, 16), sgr(16, 20)]
);

test_parse_ansi!(parse_issue_0, "│\u{1b}[0m", [text(0, 3), sgr(3, 7)]);
test_parse_ansi!(
    parse_issue_1,
    "│\u{1b}\u{1b}[0m",
    [text(0, 3), esc(3, 4), sgr(4, 8)]
);
test_parse_ansi!(
    parse_issue_3,
    "││││││││\u{1b}[0m",
    [text(0, 24), sgr(24, 28)]
);
test_parse_ansi!(parse_issue_4, "\u{1b}││││││││", [esc(0, 1), text(1, 25)]);
test_parse_ansi!(parse_issue_5, "││││││││\u{1b}", [text(0, 24), esc(24, 25)]);
test_parse_ansi!(
    parse_issue_6,
    "\u{1b}[37m│\u{1b}",
    [sgr(0, 5), text(5, 8), esc(8, 9)]
);
