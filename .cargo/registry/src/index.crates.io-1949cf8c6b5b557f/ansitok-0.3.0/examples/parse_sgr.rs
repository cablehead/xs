use ansitok::{parse_ansi, parse_ansi_sgr, ElementKind};

fn main() {
    let text = "\x1b[31;1;4mHello World\x1b[0m \x1b[38;2;255;255;0m!!!\x1b[0m";

    for element in parse_ansi(text) {
        if element.kind() != ElementKind::Sgr {
            continue;
        }

        let text = &text[element.start()..element.end()];

        println!("text={:?}", text);
        for style in parse_ansi_sgr(text) {
            println!("style={:?}", style);
        }
    }
}
