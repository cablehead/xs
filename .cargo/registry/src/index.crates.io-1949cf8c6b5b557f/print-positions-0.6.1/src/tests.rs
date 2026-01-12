#![cfg(test)]
use super::*;

#[allow(unused_imports)]
use anyhow::{anyhow, Context, Result};

fn esc_sgr_reset0() -> &'static str {
    "\x1b[0m"
}
fn esc_sgr_reset() -> &'static str {
    "\x1b[m"
}
fn esc_sgr_color() -> &'static str {
    "\x1b[1;3m"
}

// test both flavors of iterator for one scenario
fn run_test(tag: &str, expected: &[(usize, &str)], input: &[&str]) -> Result<()> {
    #[allow(unused_mut)]
    let mut test_input = input.join("");
    let mut observed: Vec<(usize, usize)> = vec![];
    let expected_indices:Vec<(usize, usize)> = expected.iter().map(|i| (i.0, i.0 + i.1.len())).collect();

    for (start, end) in print_positions(&test_input) {
        if observed.len() > 0 {
            let prev_end = observed.last().expect("length checked").1;
            assert!(
                start >= prev_end,
                "{tag}: new offset({start}) not greater than last seen ({prev_end})"
            );
        };
        assert!(end > start, "{tag}: empty substring returned");
        observed.push((start, end));
    }

    assert_eq!(expected_indices, observed, "{tag}: ");

    let mut observed: Vec<&str> = vec![];

    for substring in print_position_data(&test_input) {
        assert!(
            substring.len() > 0,
            "{tag}: empty substring returned (print_positions)"
        );
        observed.push(substring);
    }

    assert_eq!(
        expected.len(),
        observed.len(),
        "{tag}: comparing print positions iterator length"
    );
    for (exp, obs) in expected.iter().zip(observed) {
        assert_eq!(
            exp.1, obs,
            "{tag}: comparing print positions individual returns"
        );
    }

    Ok(())
}

#[test]
fn empty_string() -> Result<()> {
    run_test("", &vec![], &vec![])
}
#[test]
fn simple1() -> Result<()> {
    //let test_string = ["abc", esc_sgr_color(), "def", esc_sgr_reset0()].join("");
    let test_input = ["abc", esc_sgr_color(), "def"];
    let e1 = [esc_sgr_color(), "d"].join("");
    let expect = vec![(0, "a"), (1, "b"), (2, "c"), (3, &e1), (10, "e"), (11, "f")];

    run_test("", &expect, &test_input)
}
#[test]
fn trailing_reset() -> Result<()> {
    //let test_input = ["abc", esc_sgr_color(), "def", esc_sgr_reset0()];
    let test_input = ["ef", esc_sgr_reset0()];
    let e2 = ["f", esc_sgr_reset0()].join("");
    //let expect = vec![(0, "a"), (1, "b"), (2, "c"), (3, &e1), (10, "e"), (11, "f"), (12, &e2)];
    let expect = vec![(0, "e"), (1, &e2)];

    run_test("", &expect, &test_input)
}
#[test]
fn embedded_csi_and_trailing_reset() -> Result<()> {
    let test_input = ["abc", esc_sgr_color(), "def", esc_sgr_reset()];
    //let test_input = [ "f", esc_sgr_reset0()];
    let e1 = [esc_sgr_color(), "d"].join("");
    let e2 = ["f", esc_sgr_reset()].join("");
    let expect = vec![(0, "a"), (1, "b"), (2, "c"), (3, &e1), (10, "e"), (11, &e2)];
    //let expect = vec![(0, &e2)];

    run_test("", &expect, &test_input)
}

#[test]
fn non_reset_esc_seq_at_end_of_string() -> Result<()> {
    let test_input = ["abc", "\u{1b}\x06"]; // garbage esc seq at end of string
    let expect = vec![(0, "a"), (1, "b"), (2, "c"), (3, "\u{1b}\x06")];

    run_test("", &expect, &test_input)
}

#[test]
fn double_trailing_reset() -> Result<()> {
    let test_input = [
        "abc",
        esc_sgr_color(),
        "def",
        esc_sgr_reset(),
        esc_sgr_reset0(),
        "g",
    ];
    let e1 = [esc_sgr_color(), "d"].join("");
    let e2 = ["f", esc_sgr_reset(), esc_sgr_reset0()].join("");
    let expect = vec![
        (0, "a"),
        (1, "b"),
        (2, "c"),
        (3, &e1),
        (10, "e"),
        (11, &e2),
        (19, "g"),
    ];

    run_test("", &expect, &test_input)
}

#[test]
fn osc_termination1() -> Result<()> {
    let cases = vec![
        (
            "OSC standard termination",
            vec!["a", "\u{1b}]", "abcdef", "\u{1b}\\", "zZ"],
            vec![(0, "a"), (1, "\u{1b}]abcdef\u{1b}\\z"), (12, "Z")],
        ),
        (
            "OSC BEL termination",
            vec!["\u{1b}]", "abcdef", "\x07", "zZ"],
            vec![(0, "\u{1b}]abcdef\x07z"), (10, "Z")],
        ),
        (
            "OSC ESC but no terminator",
            vec!["\u{1b}]", "abcdef", "\u{1b}", "z"],
            vec![(0, "\u{1b}]abcdef\u{1b}z")],
        ),
        (
            "OSC ESC stuff ESC normal termination",
            vec!["\u{1b}]", "abcdef", "\u{1b}foo", "\u{1b}\\", "zZ"],
            vec![(0, "\u{1b}]abcdef\u{1b}foo\u{1b}\\z"), (15, "Z")],
        ),
        (
            "OSC ESC ESC normal",
            vec!["\u{1b}]", "abcdef", "\u{1b}\u{1b}\\", "zZ"],
            vec![(0, "\u{1b}]abcdef\u{1b}\u{1b}\\z"), (12, "Z")],
        ),
    ];

    for c in cases {
        run_test(c.0, &c.2, &c.1)?
    }

    Ok(())
}

// fuzz testing found a problem with this input: [45, 27, 91, 109, 221, 133]
// but it doesn't fail in test even with nightly compiler --release vs --test?
#[test]
fn error_from_fuzz_test() -> Result<()> {
    //let test_input = ["-\x1b[0m\u{dd}\u{85}"];
    let input = ["\u{d1}\u{97}\x1b[m\u{d2}\u{83}"];
    let expected = vec![
        (0, "\u{d1}"),
        (2, "\u{97}\x1b[m"),
        (7, "\u{d2}"),
        (9, "\u{83}"),
    ];

    run_test("", &expected, &input)
}

#[test]
fn new_line_tests() -> Result<()> {
    // unicode standard says \r\n is a single grapheme.  But separately? or \n\r?
    let input = ["\r\n", "\na\rb", "\n\r", "\r\n"];
    let expected = vec![
        (0, "\r\n"),
        (2, "\n"),
        (3, "a"),
        (4, "\r"),
        (5, "b"),
        (6, "\n"),
        (7, "\r"),
        (8, "\r\n"),
    ];

    run_test("", &expected, &input)
}

// testing the whole zoo of Unicode is somebody else's problem
// but we do test at least test some multi-byte unicode and some grapheme clusters

#[test]
fn unicode_multibyte_mixed_tests() -> Result<()> {
    // samples from UnicodeSegmentation "a̐éö̲"; // 3 bytes each

    let input = ["a", esc_sgr_color(), "a̐é", esc_sgr_reset(), esc_sgr_reset()];
    let e1 = [esc_sgr_color(), "a̐"].join("");
    let e2 = ["é", esc_sgr_reset(), esc_sgr_reset()].join("");
    let expected = vec![(0, "a"), (1, &e1), (10, &e2)];

    run_test("", &expected, &input)
}

#[test]
fn fuzz_failure_1() -> Result<()> {
    // hooray for fuzz testing!
    // it turns out the last char of a reset escape sequence
    // can form a grapheme cluster with the following chars.
    // so the reset sequence may *not* be the end of the returned grapheme.
    let input = [std::str::from_utf8(&[63, 27, 99, 217, 151]).expect("foo")];
    let expected = vec![(0, "?\u{1b}c\u{657}")];

    run_test("", &expected, &input)
}
