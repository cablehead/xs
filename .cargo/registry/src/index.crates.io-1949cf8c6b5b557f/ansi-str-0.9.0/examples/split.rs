use ansi_str::AnsiStr;

pub fn main() {
    let text = "
        It's funny how life gets complicated
        It's funny how life just takes its toll
        It's funny how everything leads to something
        Now I'm back, where I belong

        X Ambassadors - Belong
    ";

    let text = colorize(text);

    for word in text.ansi_split(" ") {
        if word.ansi_strip().is_empty() {
            continue;
        }

        println!("{word}");
    }
}

fn colorize(text: &str) -> String {
    let mut buf = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let line = if i % 2 == 0 {
            format!("\u{1b}[31;40m{line}\u{1b}[0m")
        } else if i % 3 == 0 {
            format!("\u{1b}[32;43m{line}\u{1b}[0m")
        } else {
            format!("\u{1b}[46m{line}\u{1b}[0m")
        };

        buf.push(line);
    }

    buf.join("\n")
}
