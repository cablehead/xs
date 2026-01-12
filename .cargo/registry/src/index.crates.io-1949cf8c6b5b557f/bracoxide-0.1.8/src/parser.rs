/*
 * This file is part of bracoxide.
 *
 * bracoxide is under MIT license.
 *
 * Copyright (c) 2023 A. Taha Baki <atahabaki@pm.me>
 */

//! Provides functions and types for parsing tokens into an abstract syntax tree (AST).
//!
//! ## Overview
//!
//! The parser module is responsible for transforming a sequence of tokens into a structured AST representation.
//! It takes the output of the tokenizer and performs the necessary parsing operations to generate the AST nodes.
//! The AST can then be used for further processing, interpretation, or code generation.

use std::sync::Arc;

use crate::tokenizer::Token;

/// Represents a node in the parsed AST.
///
/// The `Node` enum captures different elements in the parsed abstract syntax tree (AST).
/// It includes variants for representing text, brace expansions, and ranges.
#[derive(Debug, PartialEq, Clone)]
pub enum Node {
    /// Represents a text node.
    /// It contains the text value and the starting position of the text.
    Text { message: Arc<String>, start: usize },
    /// Represents a brace expansion node.
    /// It includes the prefix, inside, and outside node, along with the
    /// starting positions.
    BraceExpansion {
        prefix: Option<Box<Node>>,
        inside: Option<Box<Node>>,
        postfix: Option<Box<Node>>,
        start: usize,
        end: usize,
    },
    /// Represents comma seperated Nodes in braces.
    Collection {
        items: Vec<Node>,
        start: usize,
        end: usize,
    },
    /// Represents a range node.
    /// It contains the starting and ending numbers of the range, along with the
    /// starting position.
    Range {
        from: Arc<String>,
        to: Arc<String>,
        start: usize,
        end: usize,
    },
}

/// Represents an error that can occur during parsing.
///
/// The `ParsingError` enum captures different error scenarios that can happen during parsing.
#[derive(Debug, PartialEq)]
pub enum ParsingError {
    /// Indicates that there are no tokens to parse.
    NoTokens,
    /// Expected OBra, not found... e.g. `..3}` or `1..3`
    OBraExpected(usize),
    /// Expected closing bra, not fond... e.g. `{0..3` => Expected Syntax: `{0..3}`
    CBraExpected(usize),
    /// Expected Range Start number... e.g. `{...3}` or `{..3`
    RangeStartLimitExpected(usize),
    /// Expected Range Ending number... e.g. `{0..`
    RangeEndLimitExpected(usize),
    /// It is not Text, but expected to be a text.
    ExpectedText(usize),
    /// Comma is used invalid, e.g. `{A..,B}` or `{A,..B}`
    InvalidCommaUsage(usize),
    /// Extra Closing Brace, e.g. `{} }`
    ExtraCBra(usize),
    /// Extra Opening Brace, e.g. `{3{..5}`
    ExtraOBra(usize),
    /// Nothing in braces, e.g. `{}`
    NothingInBraces(usize),
    /// Range can't have text in it.
    RangeCantHaveText(usize),
    /// Extra Range Operator have used, e.g. `{3..5..}`
    ExtraRangeOperator(usize),
}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsingError::NoTokens => write!(f, "Token list is empty."),
            ParsingError::OBraExpected(i) => write!(f, "An opening brace ({{) expected at {i}"),
            ParsingError::CBraExpected(i) => write!(f, "A closing brace (}}) expected at {i}"),
            ParsingError::RangeStartLimitExpected(i) => {
                write!(f, "Range start limit not specified. Expected at {i}")
            }
            ParsingError::RangeEndLimitExpected(i) => {
                write!(f, "Range end limit not specified. Expected at {i}")
            }
            ParsingError::ExpectedText(i) => write!(f, "Expected text at {i}."),
            ParsingError::InvalidCommaUsage(i) => write!(f, "Unexpected comma at {i}"),
            ParsingError::ExtraCBra(i) => write!(f, "Used extra closing bracket at {i}"),
            ParsingError::ExtraOBra(i) => write!(f, "Used extra opening bracket at {i}"),
            ParsingError::NothingInBraces(i) => write!(
                f,
                "Empty braces at {i} causes question whether to skip it or add to tree."
            ),
            ParsingError::RangeCantHaveText(i) => write!(
                f,
                "Unrecognized char at {i}. Range syntax doesn't allow other than [0-9.]"
            ),
            ParsingError::ExtraRangeOperator(i) => {
                write!(f, "Extra range operator (..) used at {i}")
            }
        }
    }
}

impl std::error::Error for ParsingError {}

/// Parses a sequence of tokens into an abstract syntax tree (AST).
///
/// The [parse] function takes a vector of tokens as input and performs the parsing operation.
/// It returns a result with the parsed AST nodes on success, or a specific error on failure.
///
/// # Arguments
///
/// * `tokens` - A vector of tokens to be parsed.
///
/// # Returns
///
/// * `Result<Node, ParsingError>` - A result containing the parsed AST nodes or an error.
pub fn parse(tokens: &Vec<Token>) -> Result<Node, ParsingError> {
    if tokens.is_empty() {
        return Err(ParsingError::NoTokens);
    }
    match seperate(tokens) {
        Ok(seperated) => {
            let prefix = if let Some(prefix) = seperated.0 {
                match text(&prefix) {
                    Ok(n) => Some(Box::new(n)),
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            let inside = if let Some(inside) = seperated.1 {
                match collection(&inside) {
                    Ok(n) => Some(Box::new(n)),
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            let postfix = if let Some(postfix) = seperated.2 {
                let parsed = if postfix
                    .iter()
                    .any(|t| matches!(t, Token::OBra(_) | Token::CBra(_)))
                {
                    parse(&postfix)
                } else {
                    text(&postfix)
                };
                match parsed {
                    Ok(n) => Some(Box::new(n)),
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            let mut pos = (0_usize, 0_usize);
            if let Some(token) = tokens.first() {
                match token {
                    Token::OBra(s)
                    | Token::CBra(s)
                    | Token::Comma(s)
                    | Token::Text(_, s)
                    | Token::Number(_, s)
                    | Token::Range(s) => pos.0 = *s,
                }
            }
            if let Some(token) = tokens.last() {
                match token {
                    Token::OBra(s) | Token::CBra(s) | Token::Comma(s) => pos.1 = *s,
                    Token::Text(b, s) | Token::Number(b, s) => {
                        pos.1 = if b.len() == 1 { *s } else { s + b.len() };
                    }
                    Token::Range(s) => pos.1 = s + 1,
                }
            }
            Ok(Node::BraceExpansion {
                prefix,
                inside,
                postfix,
                start: pos.0,
                end: pos.1,
            })
        }
        Err(e) => Err(e),
    }
}

/// Separates the given tokens into prefix, inside, and postfix sections based on the bracing structure.
///
/// # Arguments
///
/// * `tokens` - A vector of tokens to be separated.
///
/// # Returns
///
/// Returns a result containing tuples of optional vectors representing the prefix, inside, and
/// postfix sections respectively. If the separation fails, a [ParsingError] is returned.
fn seperate(
    tokens: &Vec<Token>,
) -> Result<(Option<Vec<Token>>, Option<Vec<Token>>, Option<Vec<Token>>), ParsingError> {
    if tokens.is_empty() {
        return Err(ParsingError::NoTokens);
    }
    #[derive(Debug, PartialEq)]
    enum BracingState {
        Prefix,
        Inside,
        Postfix,
    }

    let mut count = (0_usize, 0_usize);
    let mut inside_tokens = vec![];
    let mut prefix_tokens = vec![];
    let mut postfix_tokens = vec![];
    let mut bracing_state = BracingState::Prefix;
    for token in tokens {
        match token {
            Token::OBra(_) => {
                count.0 += 1;
                match bracing_state {
                    BracingState::Prefix => {
                        bracing_state = BracingState::Inside;
                        inside_tokens.push(token.clone());
                    }
                    BracingState::Inside => inside_tokens.push(token.clone()),
                    BracingState::Postfix => postfix_tokens.push(token.clone()),
                }
            }
            Token::CBra(s) => {
                count.1 += 1;
                if count.0 < count.1 {
                    return Err(ParsingError::ExtraCBra(*s));
                }
                match bracing_state {
                    BracingState::Prefix => return Err(ParsingError::ExtraCBra(*s)),
                    BracingState::Inside => {
                        inside_tokens.push(token.clone());
                        if count.0 == count.1 {
                            bracing_state = BracingState::Postfix;
                        }
                    }
                    BracingState::Postfix => postfix_tokens.push(token.clone()),
                }
            }
            Token::Comma(s) | Token::Range(s) if bracing_state == BracingState::Prefix => {
                return Err(ParsingError::OBraExpected(*s));
            }
            _ => match bracing_state {
                BracingState::Prefix => prefix_tokens.push(token.clone()),
                BracingState::Inside => inside_tokens.push(token.clone()),
                BracingState::Postfix => postfix_tokens.push(token.clone()),
            },
        }
    }
    let prefix = if prefix_tokens.is_empty() {
        None
    } else {
        Some(prefix_tokens)
    };
    let inside = if inside_tokens.is_empty() {
        None
    } else {
        Some(inside_tokens)
    };
    let postfix = if postfix_tokens.is_empty() {
        None
    } else {
        Some(postfix_tokens)
    };
    Ok((prefix, inside, postfix))
}

/// Parses a sequence of tokens into a text node.
///
/// # Arguments
///
/// * `tokens` - A vector of tokens representing the text to be parsed.
///
/// # Returns
///
/// Returns a result containing a [Node] representing the parsed text. If the parsing fails,
/// a [ParsingError] is returned.
fn text(tokens: &Vec<Token>) -> Result<Node, ParsingError> {
    if tokens.is_empty() {
        return Err(ParsingError::NoTokens);
    }
    let mut buffer = String::new();
    let mut iter = tokens.iter();
    let mut start = 0_usize;
    if let Some(token) = iter.next() {
        match token {
            Token::OBra(s) | Token::CBra(s) | Token::Comma(s) | Token::Range(s) => {
                return Err(ParsingError::ExpectedText(*s))
            }
            Token::Text(b, s) | Token::Number(b, s) => {
                buffer.push_str(b);
                start = *s;
            }
        }
    }
    for token in iter {
        match token {
            Token::OBra(s) | Token::CBra(s) | Token::Comma(s) | Token::Range(s) => {
                return Err(ParsingError::ExpectedText(*s))
            }
            Token::Text(b, _) | Token::Number(b, _) => buffer.push_str(b),
        }
    }
    Ok(Node::Text {
        message: Arc::new(buffer),
        start,
    })
}

/// Parses a sequence of tokens into a range node.
///
/// # Arguments
///
/// * `tokens` - A vector of tokens representing the range to be parsed.
///
/// # Returns
///
/// Returns a result containing a [Node] representing the parsed range.
/// If the parsing fails, a [ParsingError] is returned.
fn range(tokens: &Vec<Token>) -> Result<Node, ParsingError> {
    if tokens.is_empty() {
        return Err(ParsingError::NoTokens);
    }
    let mut limits = (String::new(), String::new());
    let mut is_start = true;
    let mut is_first = true;
    let mut count = 0_u8;
    let mut pos = (0_usize, 0_usize);

    for token in tokens {
        match token {
            Token::OBra(s) => return Err(ParsingError::ExtraOBra(*s)),
            Token::CBra(s) => return Err(ParsingError::ExtraCBra(*s)),
            Token::Comma(s) => return Err(ParsingError::InvalidCommaUsage(*s)),
            Token::Text(_, s) => return Err(ParsingError::RangeCantHaveText(*s)),
            Token::Number(b, s) => {
                if is_first {
                    pos.0 = *s;
                    is_first = false;
                }
                match is_start {
                    true => limits.0.push_str(b),
                    false => limits.1.push_str(b),
                }
            }
            Token::Range(e) => {
                if is_first {
                    return Err(ParsingError::RangeStartLimitExpected(*e));
                }
                count += 1;
                if count != 1 {
                    return Err(ParsingError::ExtraRangeOperator(*e));
                }
                pos.1 = *e;
                is_start = false;
            }
        }
    }
    if limits.1.is_empty() {
        return Err(ParsingError::RangeEndLimitExpected(pos.1));
    }
    let len = limits.1.len();
    Ok(Node::Range {
        from: Arc::new(limits.0),
        to: Arc::new(limits.1),
        start: pos.0 - 1,
        // +1 for '.', +1 for `}`
        end: pos.1 + 2 + len,
    })
}

/// Parses a sequence of tokens into a [Node::Collection] node.
///
/// # Arguments
///
/// * `tokens` - A vector of tokens representing the collection to be parsed.
///
/// # Returns
///
/// Returns a result containing a [Node] representing the parsed collection. If the parsing fails, a [ParsingError] is returned.
fn collection(tokens: &Vec<Token>) -> Result<Node, ParsingError> {
    if tokens.is_empty() {
        return Err(ParsingError::NoTokens);
    }
    // start and end positions.
    let mut pos = (0_usize, 0_usize);
    // in the seperate function, we're dealing with `{}}` or `{{}`, no need to deal with it here.
    // count of OBra (`{`), CBra (`}`), and the seperator (`,`).
    let mut count = (0_usize, 0_usize, 0_usize);
    let mut collections: Vec<Vec<Token>> = vec![];
    let mut current = vec![];
    for token in tokens {
        match token {
            Token::Comma(s) if count.0 == (count.1 + 1) => {
                // increase the seperator count by 1.
                count.2 += 1;
                if current.is_empty() {
                    match collections.is_empty() {
                        true => current.push(Token::Text(Arc::new(String::new()), *s)),
                        // The previous token was comma.
                        false => current.push(Token::Text(Arc::new(String::new()), s - 1)),
                    }
                }
                // we dealt with if it's empty.
                // so it can't be empty.
                collections.push(current.clone());
                current.clear();
            }
            Token::Comma(_) => {
                current.push(token.clone());
            }
            Token::OBra(start) => {
                if count.0 == 0 {
                    pos.0 = *start;
                } else {
                    current.push(token.clone());
                }
                count.0 += 1;
            }
            Token::CBra(end) => {
                count.1 += 1;
                if count.0 == count.1 {
                    pos.1 = *end;
                } else {
                    current.push(token.clone());
                }
            }
            _ => current.push(token.clone()),
        }
    }
    if current.is_empty() && collections.len() == count.2 {
        current.push(Token::Text(Arc::new(String::new()), pos.1 - 1));
    }
    collections.push(current);
    let parse_collections = || -> Result<Node, ParsingError> {
        // Iterate over every collection on collections
        // If collection has `Token::OBra(_)` or `Token::CBra(_)`,
        //  parse it? How?
        //  It is better to put this collection inside parse(&collection), but is it any good?
        // Return a collection.
        let mut parsed_collections = vec![];
        for collection in collections.clone() {
            if collection
                .iter()
                .any(|t| matches!(t, Token::OBra(_) | Token::CBra(_)))
            {
                match parse(&collection) {
                    Ok(n) => parsed_collections.push(n),
                    Err(e) => return Err(e),
                }
            } else {
                parsed_collections.push(text(&collection)?);
            }
        }
        Ok(Node::Collection {
            items: parsed_collections,
            start: pos.0,
            end: pos.1,
        })
    };
    match collections.len() {
        0 => Err(ParsingError::NothingInBraces(pos.0)),
        1 => {
            // it is absolutely Text or Range
            // it can not be Collection.
            //
            // Check for `Token::Range(_)` exist or not
            // if not exist, then it's text, return text(&current)
            // if exist return range(&current)
            let collection = &collections[0];
            match collection.iter().any(|t| matches!(t, Token::Range(_))) {
                true => range(collection),
                false => match text(collection) {
                    Ok(node) => Ok(node),
                    Err(_) => parse_collections(),
                },
            }
        }
        _ => parse_collections(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::tokenizer::Token;

    #[test]
    fn test_outer_single_entry() {
        assert_eq!(
            parse(&vec![
                Token::OBra(0),
                Token::Text(Arc::new("W".into()), 1),
                Token::OBra(2),
                Token::Text(Arc::new("x".into()), 3),
                Token::Comma(4),
                Token::Text(Arc::new("y".into()), 5),
                Token::CBra(6),
                Token::CBra(7),
            ]),
            Ok(Node::BraceExpansion {
                prefix: None,
                inside: Some(Box::new(Node::Collection {
                    items: vec![Node::BraceExpansion {
                        prefix: Some(Box::new(Node::Text {
                            message: Arc::new("W".into()),
                            start: 1
                        })),
                        inside: Some(Box::new(Node::Collection {
                            items: vec![
                                Node::Text {
                                    message: Arc::new("x".into()),
                                    start: 3
                                },
                                Node::Text {
                                    message: Arc::new("y".into()),
                                    start: 5
                                }
                            ],
                            start: 2,
                            end: 6
                        })),
                        postfix: None,
                        start: 1,
                        end: 6,
                    }],
                    start: 0,
                    end: 7
                })),
                postfix: None,
                start: 0,
                end: 7
            })
        )
    }

    #[test]
    fn test_feature_empty_collection_item_at_the_end() {
        assert_eq!(
            parse(&vec![
                Token::Text(Arc::new("A".into()), 0),
                Token::OBra(1),
                Token::Text(Arc::new("B".into()), 2),
                Token::Comma(3),
                Token::Text(Arc::new("C".into()), 4),
                Token::Comma(5),
                Token::CBra(6),
            ]),
            Ok(Node::BraceExpansion {
                prefix: Some(Box::new(Node::Text {
                    message: Arc::new("A".into()),
                    start: 0
                })),
                inside: Some(Box::new(Node::Collection {
                    items: vec![
                        Node::Text {
                            message: Arc::new("B".into()),
                            start: 2
                        },
                        Node::Text {
                            message: Arc::new("C".into()),
                            start: 4
                        },
                        Node::Text {
                            message: Arc::new(String::new()),
                            start: 5
                        },
                    ],
                    start: 1,
                    end: 6
                })),
                postfix: None,
                start: 0,
                end: 6
            })
        )
    }

    #[test]
    fn test_feature_empty_collection_item_at_the_start() {
        assert_eq!(
            parse(&vec![
                Token::Text(Arc::new("A".into()), 0),
                Token::OBra(1),
                Token::Comma(2),
                Token::Text(Arc::new("B".into()), 3),
                Token::Comma(4),
                Token::Text(Arc::new("C".into()), 5),
                Token::CBra(6),
            ]),
            Ok(Node::BraceExpansion {
                prefix: Some(Box::new(Node::Text {
                    message: Arc::new("A".into()),
                    start: 0
                })),
                inside: Some(Box::new(Node::Collection {
                    items: vec![
                        Node::Text {
                            message: Arc::new(String::new()),
                            start: 2
                        },
                        Node::Text {
                            message: Arc::new("B".into()),
                            start: 3
                        },
                        Node::Text {
                            message: Arc::new("C".into()),
                            start: 5
                        },
                    ],
                    start: 1,
                    end: 6
                })),
                postfix: None,
                start: 0,
                end: 6
            })
        )
    }

    #[test]
    fn test_feature_empty_collection_item_in_the_middle() {
        assert_eq!(
            parse(&vec![
                Token::Text(Arc::new("A".into()), 0),
                Token::OBra(1),
                Token::Text(Arc::new("B".into()), 2),
                Token::Comma(3),
                Token::Comma(4),
                Token::Text(Arc::new("C".into()), 5),
                Token::CBra(6),
            ]),
            Ok(Node::BraceExpansion {
                prefix: Some(Box::new(Node::Text {
                    message: Arc::new("A".into()),
                    start: 0
                })),
                inside: Some(Box::new(Node::Collection {
                    items: vec![
                        Node::Text {
                            message: Arc::new("B".into()),
                            start: 2
                        },
                        Node::Text {
                            message: Arc::new(String::new()),
                            start: 3
                        },
                        Node::Text {
                            message: Arc::new("C".into()),
                            start: 5
                        },
                    ],
                    start: 1,
                    end: 6
                })),
                postfix: None,
                start: 0,
                end: 6
            })
        )
    }

    #[test]
    fn test_really_complex() {
        assert_eq!(
            parse(&vec![
                Token::Text(Arc::new("A".into()), 0),
                Token::OBra(1),
                Token::Text(Arc::new("B".into()), 2),
                Token::Comma(3),
                Token::Text(Arc::new("C".into()), 4),
                Token::OBra(5),
                Token::Text(Arc::new("D".into()), 6),
                Token::Comma(7),
                Token::Text(Arc::new("E".into()), 8),
                Token::CBra(9),
                Token::Text(Arc::new("F".into()), 10),
                Token::Comma(11),
                Token::Text(Arc::new("G".into()), 12),
                Token::CBra(13),
                Token::Text(Arc::new("H".into()), 14),
                Token::OBra(15),
                Token::Text(Arc::new("J".into()), 16),
                Token::Comma(17),
                Token::Text(Arc::new("K".into()), 18),
                Token::CBra(19),
                Token::Text(Arc::new("L".into()), 20),
                Token::OBra(21),
                Token::Number(Arc::new("3".into()), 22),
                Token::Range(23),
                Token::Number(Arc::new("5".into()), 25),
                Token::CBra(26),
            ]),
            Ok(Node::BraceExpansion {
                prefix: Some(Box::new(Node::Text {
                    message: Arc::new("A".into()),
                    start: 0
                })),
                inside: Some(Box::new(Node::Collection {
                    items: vec![
                        Node::Text {
                            message: Arc::new("B".into()),
                            start: 2
                        },
                        Node::BraceExpansion {
                            prefix: Some(Box::new(Node::Text {
                                message: Arc::new("C".into()),
                                start: 4
                            })),
                            inside: Some(Box::new(Node::Collection {
                                items: vec![
                                    Node::Text {
                                        message: Arc::new("D".into()),
                                        start: 6
                                    },
                                    Node::Text {
                                        message: Arc::new("E".into()),
                                        start: 8
                                    },
                                ],
                                start: 5,
                                end: 9
                            })),
                            postfix: Some(Box::new(Node::Text {
                                message: Arc::new("F".into()),
                                start: 10
                            })),
                            start: 4,
                            end: 10,
                        },
                        Node::Text {
                            message: Arc::new("G".into()),
                            start: 12
                        }
                    ],
                    start: 1,
                    end: 13
                })),
                postfix: Some(Box::new(Node::BraceExpansion {
                    prefix: Some(Box::new(Node::Text {
                        message: Arc::new("H".into()),
                        start: 14
                    })),
                    inside: Some(Box::new(Node::Collection {
                        items: vec![
                            Node::Text {
                                message: Arc::new("J".into()),
                                start: 16
                            },
                            Node::Text {
                                message: Arc::new("K".into()),
                                start: 18
                            },
                        ],
                        start: 15,
                        end: 19
                    })),
                    postfix: Some(Box::new(Node::BraceExpansion {
                        prefix: Some(Box::new(Node::Text {
                            message: Arc::new("L".into()),
                            start: 20
                        })),
                        inside: Some(Box::new(Node::Range {
                            from: Arc::new("3".into()),
                            to: Arc::new("5".into()),
                            start: 21,
                            end: 26
                        })),
                        postfix: None,
                        start: 20,
                        end: 26
                    })),
                    start: 14,
                    end: 26
                })),
                start: 0,
                end: 26
            })
        )
    }
}
