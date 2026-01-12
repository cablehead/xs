use ansitok::{parse_ansi, ElementKind};

fn main() {
    let text = "\x1b[31;1;4mHello World\x1b[0m";

    for e in parse_ansi(text) {
        match e.kind() {
            ElementKind::Text => {
                println!("Got a text: {:?}", &text[e.range()],);
            }
            _ => {
                println!(
                    "Got an escape sequence: {:?} from {:#?} to {:#?}",
                    e.kind(),
                    e.start(),
                    e.end()
                );
            }
        }
    }
}
