//! Demonstrating the convenience of print position length arithmetic
//! when used for padding or filling fixed width fields
//! for display on a screen with monospace fonts and unicode + emoji support.

use anyhow::Result;
use print_positions::print_position_data;

fn pad_field<'a>(components: &[&'a str], width: usize, fill: &str) {
    let padding = fill.repeat(width);
    let content = components.join("");
    let segments: Vec<_> = print_position_data(&content).collect();

    assert_eq!(
        content,
        segments.join(""),
        "print position segmentation doesn't lose / insert characters from source string"
    );

    println!(
        "characters {}  == print position {}",
        components.join(" + "),
        content
    );
    println!(
        "Content is {} chars long but {} print positions wide",
        content.len(),
        segments.len(),
    );
    println!("   centering in field padded to width {width} with `{fill}`");
    let pad_width = width - segments.len();
    let left_pad_width = pad_width / 2;
    let right_pad_width = pad_width - left_pad_width;

    println!(
        "    {}{}{}",
        &padding[..left_pad_width],
        content,
        &padding[..right_pad_width]
    );
    println!("    {}", padding);
}

fn main() -> Result<()> {
    /*
     Emoji combined with zero-width joiner (ZWJ) are powerful demos of grapheme clusters.
     Sadly, they don't render as single characters in linux terminal (at least for me),
     though they do render properly in [rust playground](https://play.rust-lang.org/)
       \u{1F468} (man) \u{200D} (zero-width joiner) \u{1F469} (woman) \u{200D} \u{1F467} (child) == (family)
     or
       \u{1f468} (man) \u{200d} (zero-width joiner) \u{1f4bb} (laptop) == (hacker)

     therefore, the default example is the rather tame combining dieresis
       \u{0067} \u{0308} == \u{0067}\u{0308}
    */
    println!("\nExample of combining dieresis");
    pad_field(&["\u{0065}", "\u{0308}"], 5, "+");

    println!("\nSame content with ANSI color embellishment");
    pad_field(
        &["\u{1b}[30;42m", "\u{0065}", "\u{0308}", "\u{1b}[0m"],
        5,
        "+",
    );

    println!("\n\nExample of emoji with ZWJ (may not work in terminal, try wasm");
    pad_field(&["\u{1f468}", "\u{200d}", "\u{1f4bb}"], 5, "+");

    println!("\n\nSame content with ANSI color embellishment");
    pad_field(
        &[
            "\u{1b}[30;42m",
            "\u{1f468}",
            "\u{200d}",
            "\u{1f4bb}",
            "\u{1b}[0m",
        ],
        5,
        "+",
    );

    Ok(())
}
