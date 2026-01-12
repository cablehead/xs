pub(crate) struct Tokenizer {
    token_stack: Vec<String>,
    // TODO: Should this be more generic? io::Read for example?
    parse_string: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ParseState {
    Empty,
    Alpha,
    AlphaDecimal,
    Numeric,
    NumericDecimal,
}

impl Tokenizer {
    pub(crate) fn new(parse_string: &str) -> Self {
        Tokenizer {
            token_stack: vec![],
            parse_string: parse_string.chars().rev().collect(),
        }
    }

    fn isword(&self, c: char) -> bool {
        c.is_alphabetic()
    }

    fn isnum(&self, c: char) -> bool {
        c.is_numeric()
    }

    fn isspace(&self, c: char) -> bool {
        c.is_whitespace()
    }

    fn decimal_split(&self, s: &str) -> Vec<String> {
        // Handles the same thing as Python's re.split()
        let mut tokens: Vec<String> = vec!["".to_owned()];

        for c in s.chars() {
            if c == '.' || c == ',' {
                tokens.push(c.to_string());
                tokens.push("".to_owned());
            } else {
                // UNWRAP: Initial setup guarantees we always have an item
                let mut t = tokens.pop().unwrap();
                t.push(c);
                tokens.push(t);
            }
        }

        // TODO: Do I really have to use &String instead of &str?
        if tokens.last() == Some(&"".to_owned()) {
            tokens.pop();
        }

        tokens
    }
}

impl Iterator for Tokenizer {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.token_stack.is_empty() {
            return Some(self.token_stack.remove(0));
        }

        let mut seenletters = false;
        let mut token: Option<String> = None;
        let mut state = ParseState::Empty;

        while !self.parse_string.is_empty() {
            // Dateutil uses a separate `charstack` to manage the incoming stream.
            // Because parse_string can have things pushed back onto it, we skip
            // a couple of steps related to the `charstack`.

            // UNWRAP: Just checked that parse_string isn't empty
            let nextchar = self.parse_string.pop().unwrap();

            match state {
                ParseState::Empty => {
                    token = Some(nextchar.to_string());
                    if self.isword(nextchar) {
                        state = ParseState::Alpha;
                    } else if self.isnum(nextchar) {
                        state = ParseState::Numeric;
                    } else if self.isspace(nextchar) {
                        token = Some(" ".to_owned());
                        break;
                    } else {
                        break;
                    }
                }
                ParseState::Alpha => {
                    seenletters = true;
                    if self.isword(nextchar) {
                        // UNWRAP: Because we're in non-empty parse state, we're guaranteed to have a token
                        token.as_mut().unwrap().push(nextchar);
                    } else if nextchar == '.' {
                        token.as_mut().unwrap().push(nextchar);
                        state = ParseState::AlphaDecimal;
                    } else {
                        self.parse_string.push(nextchar);
                        break;
                    }
                }
                ParseState::Numeric => {
                    if self.isnum(nextchar) {
                        // UNWRAP: Because we're in non-empty parse state, we're guaranteed to have a token
                        token.as_mut().unwrap().push(nextchar);
                    } else if nextchar == '.'
                        || (nextchar == ',' && token.as_ref().unwrap().len() >= 2)
                    {
                        token.as_mut().unwrap().push(nextchar);
                        state = ParseState::NumericDecimal;
                    } else {
                        self.parse_string.push(nextchar);
                        break;
                    }
                }
                ParseState::AlphaDecimal => {
                    seenletters = true;
                    if nextchar == '.' || self.isword(nextchar) {
                        // UNWRAP: Because we're in non-empty parse state, we're guaranteed to have a token
                        token.as_mut().unwrap().push(nextchar);
                    } else if self.isnum(nextchar) && token.as_ref().unwrap().ends_with('.') {
                        token.as_mut().unwrap().push(nextchar);
                        state = ParseState::NumericDecimal;
                    } else {
                        self.parse_string.push(nextchar);
                        break;
                    }
                }
                ParseState::NumericDecimal => {
                    if nextchar == '.' || self.isnum(nextchar) {
                        // UNWRAP: Because we're in non-empty parse state, we're guaranteed to have a token
                        token.as_mut().unwrap().push(nextchar);
                    } else if self.isword(nextchar) && token.as_ref().unwrap().ends_with('.') {
                        token.as_mut().unwrap().push(nextchar);
                        state = ParseState::AlphaDecimal;
                    } else {
                        self.parse_string.push(nextchar);
                        break;
                    }
                }
            }
        }

        // Python uses the state to short-circuit and make sure it doesn't run into issues with None
        // We do something slightly different to express the same logic
        if state == ParseState::AlphaDecimal || state == ParseState::NumericDecimal {
            // UNWRAP: The state check guarantees that we have a value
            let dot_count = token
                .as_ref()
                .unwrap()
                .chars()
                .filter(|c| *c == '.')
                .count();
            let last_char = token.as_ref().unwrap().chars().last();
            let last_splittable = last_char == Some('.') || last_char == Some(',');

            if seenletters || dot_count > 1 || last_splittable {
                let mut l = self.decimal_split(token.as_ref().unwrap());
                let remaining = l.split_off(1);

                token = Some(l[0].clone());
                for t in remaining {
                    self.token_stack.push(t);
                }
            }

            if state == ParseState::NumericDecimal && dot_count == 0 {
                token = Some(token.unwrap().replace(',', "."));
            }
        }

        token
    }
}

#[cfg(test)]
mod tests {

    use Tokenizer;

    #[test]
    fn test_basic() {
        let tokens: Vec<String> = Tokenizer::new("September of 2003,").collect();
        assert_eq!(tokens, vec!["September", " ", "of", " ", "2003", ","]);
    }
}
