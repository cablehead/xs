use ansi_str::AnsiStr;

fn main() {
    let text = "\u{1b}[31;40mHello\u{1b}[0m \u{1b}[32;43mWorld\u{1b}[0m";

    let (left, right) = text.ansi_split_at(6);

    println!("text={text}");
    println!("left={left}");
    println!("left={right}");
}
