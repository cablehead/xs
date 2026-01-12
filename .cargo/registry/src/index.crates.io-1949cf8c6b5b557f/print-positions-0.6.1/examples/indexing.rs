//! Demonstrating the convenience of print position indexing a source string.

use anyhow::Result;
use print_positions::print_positions;

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

    println!("Example indexing through 5 print positions.");
    let source = [
        "a ",
        "\u{1b}[30;42m",
        "\u{1f468}",
        "\u{200d}",
        "\u{1f4bb}",
        "\u{1b}[0m",
        " b",
    ]
    .join("");

    let ranges:Vec<(usize, usize)> = print_positions(&source).collect();

    for i in 0..ranges.len() {
        println!(
            "Print position[{i}]: `{}`, source[{}..{}]",
            &source[(ranges[i].0)..(ranges[i].1)],
            ranges[i].0,
            ranges[i].1
        );
    }

    Ok(())
}
