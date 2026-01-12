/*
 * This file is part of bracoxide.
 *
 * bracoxide is under MIT license.
 *
 * Copyright (c) 2023 A. Taha Baki <atahabaki@pm.me>
 */

//! Provides functionality for tokenizing input strings. It defines the
//! [Token] enum, which represents individual tokens produced during tokenization, and the
//! [tokenize] function, which converts an input string into a sequence of tokens.
//!
//! ## Usage
//!
//! To tokenize a string, use the [tokenize] function. It takes an input string as a parameter
//! and returns a `Result<Vec<Token>, TokenizationError>`. If successful, it returns a vector
//! of tokens representing the input string. If an error occurs during tokenization, it returns
//! a [TokenizationError] indicating the specific error encountered.
//!
//! The [Token] enum represents different types of tokens, such as opening braces, closing braces,
//! commas, text, numbers, and ranges. Each variant of the enum provides additional information
//! related to the token, such as the position of the token in the input string.

use std::sync::Arc;

/// Defines the possible types of tokens that can be encountered during the process of
/// tokenization.
///
/// The [Token] enum is used to represent different types of tokens that can be produced
/// while tokenizing a string. Each variant of the [Token] enum corresponds to a specific
/// type of token, such as opening brace, closing brace, comma, text, number, or range
/// operator.
///
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// Represents an opening brace `{` at the specified position.
    OBra(usize),
    /// Represents a closing brace `}` at the specified position.
    CBra(usize),
    /// Represents a comma `,` at the specified position.
    Comma(usize),
    /// Represents any non-number text at the specified position.
    ///
    /// The associated `String` contains the text value.
    Text(Arc<String>, usize),
    /// Represents a number at the specified position.
    ///
    /// The associated `String` contains the numeric value.
    Number(Arc<String>, usize),
    /// Represents the range operator `..` at the specified position.
    Range(usize),
}

/// Represents the possible errors that can occur during the tokenization.
///
/// # Example
///
/// ```rust
/// use bracoxide::tokenizer::TokenizationError;
///
/// let content = "{a, b, c, d";
/// let tokenization_result = bracoxide::tokenizer::tokenize(content);
/// assert_eq!(tokenization_result, Err(TokenizationError::FormatNotSupported));
/// ```
#[derive(Debug, PartialEq, Clone)]
pub enum TokenizationError {
    /// The content to be tokenized is empty.
    EmptyContent,
    /// The input content has an unsupported format (e.g., only an opening brace or closing
    /// brace).
    FormatNotSupported,
    /// The input content does not contain any braces.
    NoBraces,
}

impl std::fmt::Display for TokenizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenizationError::EmptyContent => write!(f, "Content is empty."),
            TokenizationError::FormatNotSupported => {
                write!(f, "Only opening brace or closing brace is used.")
            }
            TokenizationError::NoBraces => write!(f, "No braces have been used."),
        }
    }
}

impl std::error::Error for TokenizationError {}

/// Tokenizes the provided content string and produces a vector of tokens.
///
/// This function is part of the `bracoxide` crate and is used to tokenize a given string `content`.
/// The tokenization process splits the string into meaningful units called tokens, which can be
/// further processed or analyzed as needed.
///
/// # Arguments
///
/// * `content` - The string to be tokenized.
///
/// # Returns
///
/// * `Result<Vec<Token>, TokenizationError>` - A result that contains a vector of tokens if the tokenization
///   is successful, or a [TokenizationError] if an error occurs during the tokenization process.
///
/// # Errors
///
/// The function can return the following errors:
///
/// * [TokenizationError::EmptyContent] - If the `content` string is empty.
/// * [TokenizationError::NoBraces] - If the `content` string does not contain any braces.
/// * [TokenizationError::FormatNotSupported] - If the `content` string has an unsupported format, such as
///   only an opening brace or closing brace without a corresponding pair.
///
/// # Examples
///
/// ```
/// use bracoxide::tokenizer::{Token, TokenizationError, tokenize};
///
/// let content = "{1, 2, 3}";
/// let tokens = tokenize(content);
///
/// match tokens {
///     Ok(tokens) => {
///         println!("Tokenization successful!");
///         for token in tokens {
///             println!("{:?}", token);
///         }
///     }
///     Err(error) => {
///         eprintln!("Tokenization failed: {:?}", error);
///     }
/// }
/// ```
///
/// In this example, the `tokenize` function from the `bracoxide` crate is used to tokenize the content string "{1, 2, 3}".
/// If the tokenization is successful, the resulting tokens are printed. Otherwise, the corresponding error is displayed.
pub fn tokenize(content: &str) -> Result<Vec<Token>, TokenizationError> {
    if content.is_empty() {
        return Err(TokenizationError::EmptyContent);
    }
    let mut tokens = Vec::<Token>::new();
    let mut is_escape = false;
    // opening, closing
    let mut count = (0_usize, 0_usize);
    // text_buffer, number_buffer
    let mut buffers = (String::new(), String::new());
    let mut iter = content.chars().enumerate();
    let tokenize_text_buffer = |tokens: &mut Vec<Token>, buffers: &mut (String, String), i| {
        if !buffers.0.is_empty() {
            tokens.push(Token::Text(
                Arc::new(buffers.0.clone()),
                i - buffers.0.len(),
            ));
            buffers.0.clear();
        }
    };
    let tokenize_number_buffer = |tokens: &mut Vec<Token>, buffers: &mut (String, String), i| {
        if !buffers.1.is_empty() {
            tokens.push(Token::Number(
                Arc::new(buffers.1.clone()),
                i - buffers.1.len(),
            ));
            buffers.1.clear();
        }
    };
    // Push buffers into tokens.
    let tokenize_buffers = |tokens: &mut Vec<Token>, buffers: &mut (String, String), i| {
        tokenize_text_buffer(tokens, buffers, i);
        tokenize_number_buffer(tokens, buffers, i);
    };
    while let Some((i, c)) = iter.next() {
        match (c, is_escape) {
            (_, true) => {
                if !buffers.1.is_empty() {
                    buffers.0.push_str(&buffers.1);
                    buffers.1.clear();
                }
                buffers.0.push(c);
                buffers.1.clear();
                is_escape = false;
            }
            ('\\', false) => is_escape = true,
            // @1: COMMENT
            // Look it is '{' OR '}' OR ','
            // No other c value can pass this match ARM
            // And now look to @2
            ('{' | '}' | ',', _) => {
                tokenize_buffers(&mut tokens, &mut buffers, i);
                match c {
                    '{' => {
                        count.0 += 1;
                        tokens.push(Token::OBra(i));
                    }
                    '}' => {
                        count.1 += 1;
                        tokens.push(Token::CBra(i));
                    }
                    ',' => tokens.push(Token::Comma(i)),
                    // @2: COMMENT
                    // Look @1 the above catch, you see
                    // c can be just '{' OR '}' OR ','.
                    // AND Why the god damn rust wants me to handle all cases,
                    // Where I got covered all cases above.
                    _ => unreachable!(),
                }
            }
            ('.', _) => {
                let mut r_iter = iter.clone();
                if let Some((_ix, cx)) = r_iter.next() {
                    match cx {
                        '.' if count.0 == count.1 => {
                            buffers.0.push(c);
                            buffers.0.push(cx);
                            tokenize_buffers(&mut tokens, &mut buffers, i + 2);
                            iter = r_iter;
                        }
                        '.' => {
                            tokenize_buffers(&mut tokens, &mut buffers, i);
                            tokens.push(Token::Range(i));
                            iter = r_iter;
                            continue;
                        }
                        _ => {
                            tokenize_number_buffer(&mut tokens, &mut buffers, i);
                            buffers.0.push(c);
                        }
                    }
                } else {
                    buffers.0.push(c);
                }
            }
            ('0'..='9', _) => {
                tokenize_text_buffer(&mut tokens, &mut buffers, i);
                buffers.1.push(c);
            }
            _ => {
                tokenize_number_buffer(&mut tokens, &mut buffers, i);
                buffers.0.push(c);
            }
        }
    }
    match count {
        (0, 0) => return Err(TokenizationError::NoBraces),
        (0, _) | (_, 0) => return Err(TokenizationError::FormatNotSupported),
        (_, _) => (),
    }
    tokenize_buffers(&mut tokens, &mut buffers, content.len());
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        assert_eq!(tokenize(""), Err(TokenizationError::EmptyContent));
        assert_eq!(
            tokenize(String::new().as_str()),
            Err(TokenizationError::EmptyContent)
        );
    }

    #[test]
    fn test_double_dots_noerror() {
        assert_eq!(
            tokenize("..{a,b}",),
            Ok(vec![
                Token::Text(Arc::new("..".to_string()), 0),
                Token::OBra(2),
                Token::Text(Arc::new("a".to_string()), 3),
                Token::Comma(4),
                Token::Text(Arc::new("b".to_string()), 5),
                Token::CBra(6),
            ])
        )
    }

    #[test]
    fn test_no_braces() {
        assert_eq!(tokenize("a"), Err(TokenizationError::NoBraces));
        assert_eq!(tokenize("1..3"), Err(TokenizationError::NoBraces));
        assert_eq!(tokenize("a,b"), Err(TokenizationError::NoBraces));
        assert_eq!(
            tokenize("arst1..3.(arst)xt"),
            Err(TokenizationError::NoBraces)
        );
    }

    #[test]
    fn test_format_not_supported() {
        assert_eq!(
            tokenize("{a, b, c, d"),
            Err(TokenizationError::FormatNotSupported)
        );
        assert_eq!(
            tokenize("{{a, b, c, d"),
            Err(TokenizationError::FormatNotSupported)
        );
        assert_eq!(
            tokenize("a, b, c, d}}"),
            Err(TokenizationError::FormatNotSupported)
        );
        assert_eq!(
            tokenize("a{, b{, c{, d{"),
            Err(TokenizationError::FormatNotSupported)
        );
    }

    #[test]
    fn test_tokenize_single_brace_expansion() {
        let content = "A{1..3}";
        let expected_result: Result<Vec<Token>, TokenizationError> = Ok(vec![
            Token::Text(Arc::new("A".to_string()), 0),
            Token::OBra(1),
            Token::Number(Arc::new("1".to_string()), 2),
            Token::Range(3),
            Token::Number(Arc::new("3".to_string()), 5),
            Token::CBra(6),
        ]);
        assert_eq!(tokenize(content), expected_result);
        let content = "{AB12}";
        let expected_result: Result<Vec<Token>, TokenizationError> = Ok(vec![
            Token::OBra(0),
            Token::Text(Arc::new("AB".to_string()), 1),
            Token::Number(Arc::new("12".to_string()), 3),
            Token::CBra(5),
        ]);
        assert_eq!(tokenize(content), expected_result);
        let content = "{12AB}";
        let expected_result: Result<Vec<Token>, TokenizationError> = Ok(vec![
            Token::OBra(0),
            Token::Number(Arc::new("12".to_string()), 1),
            Token::Text(Arc::new("AB".to_string()), 3),
            Token::CBra(5),
        ]);
        assert_eq!(tokenize(content), expected_result);
    }

    #[test]
    fn test_tokenize_multiple_brace_expansions() {
        let content = "A{1,2}..B{3,4}";
        let expected_result: Result<Vec<Token>, TokenizationError> = Ok(vec![
            Token::Text(Arc::new("A".to_string()), 0),
            Token::OBra(1),
            Token::Number(Arc::new("1".to_string()), 2),
            Token::Comma(3),
            Token::Number(Arc::new("2".to_string()), 4),
            Token::CBra(5),
            Token::Text(Arc::new("..".to_string()), 6),
            Token::Text(Arc::new("B".to_string()), 8),
            Token::OBra(9),
            Token::Number(Arc::new("3".to_string()), 10),
            Token::Comma(11),
            Token::Number(Arc::new("4".to_string()), 12),
            Token::CBra(13),
        ]);
        assert_eq!(tokenize(content), expected_result);
    }

    #[test]
    fn test_tokenize() {
        // Test case 1: {1..3}
        assert_eq!(
            tokenize("{1..3}"),
            Ok(vec![
                Token::OBra(0),
                Token::Number(Arc::new("1".to_owned()), 1),
                Token::Range(2),
                Token::Number(Arc::new("3".to_owned()), 4),
                Token::CBra(5)
            ])
        );

        // Test case 2: {a,b,c}
        assert_eq!(
            tokenize("{a,b,c}"),
            Ok(vec![
                Token::OBra(0),
                Token::Text(Arc::new("a".to_owned()), 1),
                Token::Comma(2),
                Token::Text(Arc::new("b".to_owned()), 3),
                Token::Comma(4),
                Token::Text(Arc::new("c".to_owned()), 5),
                Token::CBra(6)
            ])
        );

        // Test case 12: A{1..3}..B{2,5}
        assert_eq!(
            tokenize("A{1..3}..B{2,5}"),
            Ok(vec![
                Token::Text(Arc::new("A".to_owned()), 0),
                Token::OBra(1),
                Token::Number(Arc::new("1".to_owned()), 2),
                Token::Range(3),
                Token::Number(Arc::new("3".to_owned()), 5),
                Token::CBra(6),
                Token::Text(Arc::new("..".to_owned()), 7),
                Token::Text(Arc::new("B".to_owned()), 9),
                Token::OBra(10),
                Token::Number(Arc::new("2".to_owned()), 11),
                Token::Comma(12),
                Token::Number(Arc::new("5".to_owned()), 13),
                Token::CBra(14)
            ])
        );
    }

    #[test]
    fn test_dots() {
        assert_eq!(
            tokenize("{1..3}"),
            Ok(vec![
                Token::OBra(0),
                Token::Number(Arc::new("1".to_owned()), 1),
                Token::Range(2),
                Token::Number(Arc::new("3".to_owned()), 4),
                Token::CBra(5),
            ])
        );
        assert_eq!(
            tokenize("{1.2.3,b}"),
            Ok(vec![
                Token::OBra(0),
                Token::Number(Arc::new("1".to_owned()), 1),
                Token::Text(Arc::new(".".to_owned()), 2),
                Token::Number(Arc::new("2".to_owned()), 3),
                Token::Text(Arc::new(".".to_owned()), 4),
                Token::Number(Arc::new("3".to_owned()), 5),
                Token::Comma(6),
                Token::Text(Arc::new("b".to_owned()), 7),
                Token::CBra(8),
            ])
        );
        assert_eq!(
            tokenize("{a.b.c,d}"),
            Ok(vec![
                Token::OBra(0),
                Token::Text(Arc::new("a.b.c".to_owned()), 1),
                Token::Comma(6),
                Token::Text(Arc::new("d".to_owned()), 7),
                Token::CBra(8),
            ])
        );
    }

    #[test]
    fn test_numbers_with_proceeding_escapees_are_text_now() {
        assert_eq!(
            tokenize("1\\\\{a,b}"),
            Ok(vec![
                Token::Text(Arc::new("1\\".into()), 1),
                Token::OBra(3),
                Token::Text(Arc::new("a".into()), 4),
                Token::Comma(5),
                Token::Text(Arc::new("b".into()), 6),
                Token::CBra(7),
            ])
        );
        assert_eq!(
            tokenize("1\\a{b,c}"),
            Ok(vec![
                Token::Text(Arc::new("1a".into()), 1),
                Token::OBra(3),
                Token::Text(Arc::new("b".into()), 4),
                Token::Comma(5),
                Token::Text(Arc::new("c".into()), 6),
                Token::CBra(7),
            ])
        );
        assert_eq!(
            tokenize("{1\\2,3\\\\{4\\5,6\\7}}"),
            Ok(vec![
                Token::OBra(0),
                Token::Text(Arc::new("12".into()), 2),
                Token::Comma(4),
                Token::Text(Arc::new("3\\".into()), 6),
                Token::OBra(8),
                Token::Text(Arc::new("45".into()), 10),
                Token::Comma(12),
                Token::Text(Arc::new("67".into()), 14),
                Token::CBra(16),
                Token::CBra(17),
            ])
        );
    }
}
