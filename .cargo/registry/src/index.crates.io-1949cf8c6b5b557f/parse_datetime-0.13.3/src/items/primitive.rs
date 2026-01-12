// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

//! Primitive combinators.

use std::str::FromStr;

use winnow::{
    ascii::{digit1, multispace0, Uint},
    combinator::{alt, delimited, not, opt, peek, preceded, repeat, separated},
    error::{ContextError, ParserError, StrContext, StrContextValue},
    stream::AsChar,
    token::{none_of, one_of, take_while},
    Parser,
};

/// Allow spaces and comments before a parser
///
/// Every token parser should be wrapped in this to allow spaces and comments.
/// It is only preceding, because that allows us to check mandatory whitespace
/// after running the parser.
pub(super) fn s<'a, O, E>(p: impl Parser<&'a str, O, E>) -> impl Parser<&'a str, O, E>
where
    E: ParserError<&'a str>,
{
    preceded(space, p)
}

/// Parse the space in-between tokens
///
/// You probably want to use the [`s`] combinator instead.
pub(super) fn space<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    separated(0.., multispace0, alt((comment, ignored_hyphen_or_plus))).parse_next(input)
}

/// A hyphen or plus is ignored when it is not followed by a digit
///
/// This includes being followed by a comment! Compare these inputs:
/// ```txt
/// - 12 weeks
/// - (comment) 12 weeks
/// ```
/// The last comment should be ignored.
///
/// The plus is undocumented, but it seems to be ignored.
fn ignored_hyphen_or_plus<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    (
        alt(('-', '+')),
        multispace0,
        peek(not(take_while(1, AsChar::is_dec_digit))),
    )
        .void()
        .parse_next(input)
}

/// Parse a comment
///
/// A comment is given between parentheses, which must be balanced. Any other
/// tokens can be within the comment.
fn comment<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    delimited(
        '(',
        repeat(0.., alt((none_of(['(', ')']).void(), comment))),
        ')',
    )
    .parse_next(input)
}

/// Parse a signed decimal integer.
///
/// Rationale for not using `winnow::ascii::dec_int`: When upgrading winnow from
/// 0.5 to 0.7, we discovered that `winnow::ascii::dec_int` now accepts only the
/// following two forms:
///
/// - 0
/// - [+-]?[1-9][0-9]*
///
/// Inputs like [+-]?0[0-9]* (e.g., `+012`) are therefore rejected. We provide a
/// custom implementation to support such zero-prefixed integers.
#[allow(unused)]
pub(super) fn dec_int<'a, E>(input: &mut &'a str) -> winnow::Result<i32, E>
where
    E: ParserError<&'a str>,
{
    (opt(one_of(['+', '-'])), digit1)
        .void()
        .take()
        .verify_map(|s: &str| s.parse().ok())
        .parse_next(input)
}

/// Parse an unsigned decimal integer.
///
/// See the rationale for `dec_int` for why we don't use
/// `winnow::ascii::dec_uint`.
pub(super) fn dec_uint<'a, O, E>(input: &mut &'a str) -> winnow::Result<O, E>
where
    O: Uint + FromStr,
    E: ParserError<&'a str>,
{
    dec_uint_str
        .verify_map(|s: &str| s.parse().ok())
        .parse_next(input)
}

/// Parse an unsigned decimal integer as a string slice.
pub(super) fn dec_uint_str<'a, E>(input: &mut &'a str) -> winnow::Result<&'a str, E>
where
    E: ParserError<&'a str>,
{
    digit1.void().take().parse_next(input)
}

/// Parse a colon preceded by whitespace.
pub(super) fn colon<'a, E>(input: &mut &'a str) -> winnow::Result<(), E>
where
    E: ParserError<&'a str>,
{
    s(':').void().parse_next(input)
}

/// Parse a plus or minus character optionally preceeded by whitespace.
pub(super) fn plus_or_minus<'a, E>(input: &mut &'a str) -> winnow::Result<char, E>
where
    E: ParserError<&'a str>,
{
    s(alt(('+', '-'))).parse_next(input)
}

/// Create a context error with a reason.
pub(super) fn ctx_err(reason: &'static str) -> ContextError {
    let mut err = ContextError::new();
    err.push(StrContext::Expected(StrContextValue::Description(reason)));
    err
}
