use ansi_str::AnsiStr;

pub fn main() {
    let text = "\u{1b}[1m\u{1b}[31;46mWhen the night has come\u{1b}[0m\u{1b}[0m";

    let slice = text.ansi_get(5..).expect("ok");

    println!("text={text}");
    println!("slice={slice}");
}
