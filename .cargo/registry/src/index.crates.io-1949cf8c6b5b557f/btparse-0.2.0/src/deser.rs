//! Deserialization helper functions
use crate::{Error, Frame, Kind};

pub(crate) fn close_bracket(bt: &str) -> Result<&str, Error> {
    start(bt, "]")
}

pub(crate) fn frame(bt: &str) -> Result<(&str, Frame), Error> {
    let (bt, frame) = delimited(bt, "{", "}")?;
    let frame = start(frame, "fn: ")?;
    let file_match = ", file: ";
    let file_start = frame.find(file_match);
    let line_match = ", line: ";
    let line_start = frame.find(line_match);

    let fn_end = file_start.or(line_start).unwrap_or_else(|| frame.len());
    let function = frame[..fn_end].trim_matches('"').to_string();

    let file = file_start
        .map(|start| {
            (
                start + file_match.len(),
                line_start.unwrap_or_else(|| frame.len()),
            )
        })
        .map(|(start, end)| &frame[start..end])
        .map(|file| file.trim_matches('"'))
        .map(ToString::to_string);

    let line = line_start
        .map(|start| (start + line_match.len(), frame.len()))
        .map(|(start, end)| &frame[start..end])
        .map(|line| {
            line.parse::<usize>()
                .map_err(|source| Kind::LineParse(line.into(), source))
        })
        .transpose()?;

    Ok((
        bt,
        Frame {
            function,
            line,
            file,
        },
    ))
}

fn delimited<'a>(bt: &'a str, start: &str, end: &str) -> Result<(&'a str, &'a str), Error> {
    let mut depth = 1;

    let start_len = start.chars().count();

    if !bt.starts_with(start) {
        Err(Kind::InvalidInput {
            expected: start.into(),
            found: bt.chars().take(start_len).collect(),
        })?;
    }

    let start_ind = start_len;
    let mut end_ind = None;

    for (ind, _) in bt.char_indices().skip(start_len) {
        match bt.get(ind..) {
            Some(next) if next.starts_with(start) => depth += 1,
            Some(next) if next.starts_with(end) => depth -= 1,
            Some(_) => (),
            None => unimplemented!(),
        }

        if depth == 0 {
            end_ind = Some(ind);
            break;
        }
    }

    if let Some(end_ind) = end_ind {
        let end = end_ind + end.len();
        Ok((&bt[end..], &bt[start_ind..end_ind].trim()))
    } else {
        unimplemented!()
    }
}

pub(crate) fn header(bt: &str) -> Result<&str, Error> {
    let mut parts = bt.splitn(2, '[');
    let header = parts.next().unwrap();

    match header {
        "Backtrace " => (),
        "<disabled>" | "disabled backtrace" => Err(Kind::Disabled)?,
        "unsupported backtrace" => Err(Kind::Unsupported)?,
        _ => Err(Kind::UnexpectedInput(header.into()))?,
    }

    Ok(parts.next().unwrap())
}

fn start<'a>(bt: &'a str, expected: &str) -> Result<&'a str, Error> {
    if bt.starts_with(expected) {
        return Ok(bt.trim_start_matches(expected));
    }

    Err(Kind::InvalidInput {
        found: bt.chars().take(expected.chars().count()).collect(),
        expected: expected.to_string(),
    })?
}

pub(crate) fn trailing_comma(bt: &str) -> (&str, bool) {
    start(bt, ", ").map(|bt| (bt, true)).unwrap_or((bt, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_test() -> eyre::Result<()> {
        let input = "{fn: \"function_name\", file: \"file_name\", line: 10} extra input";
        let (input, f) = frame(input)?;
        assert_eq!(" extra input", input);
        assert_eq!(
            Frame {
                function: "function_name".into(),
                file: Some("file_name".into()),
                line: Some(10)
            },
            f
        );

        let input = "{fn: \"function_name\", line: 10} extra input";
        let (input, f) = frame(input)?;
        assert_eq!(" extra input", input);
        assert_eq!(
            Frame {
                function: "function_name".into(),
                file: None,
                line: Some(10)
            },
            f
        );

        let input = "{fn: \"function_name\", file: \"file_name\"} extra input";
        let (input, f) = frame(input)?;
        assert_eq!(" extra input", input);
        assert_eq!(
            Frame {
                function: "function_name".into(),
                file: Some("file_name".into()),
                line: None,
            },
            f
        );

        let input = "{fn: \"function_name\"} extra input";
        let (input, f) = frame(input)?;
        assert_eq!(" extra input", input);
        assert_eq!(
            Frame {
                function: "function_name".into(),
                file: None,
                line: None,
            },
            f
        );

        let input = "{n: \"function_name\"} extra input";
        let e = frame(input).unwrap_err();
        match e {
            Error {
                kind: Kind::InvalidInput { found, .. },
                ..
            } => {
                assert_eq!(found, "n: \"");
            }
            _ => Err(e)?,
        }

        Ok(())
    }

    #[test]
    fn start_test() -> eyre::Result<()> {
        let input = "hi whatsup";
        let input = start(input, "hi ")?;
        assert_eq!("whatsup", input);

        Ok(())
    }

    #[test]
    fn delimited_test() -> eyre::Result<()> {
        let input = "{this is delimited by {} some garbage} the remainder";
        let (input, delimited_bit) = delimited(input, "{", "}")?;
        assert_eq!("this is delimited by {} some garbage", delimited_bit);
        assert_eq!(" the remainder", input);

        Ok(())
    }

    #[test]
    fn trailing_comma_test() -> eyre::Result<()> {
        let input = ", hi whatsup";
        let (input, had) = trailing_comma(input);
        assert!(had);
        let (_, had) = trailing_comma(input);
        assert!(!had);

        Ok(())
    }

    #[test]
    fn header_test() -> eyre::Result<()> {
        let input = header("Backtrace [")?;
        assert!(input.is_empty());

        match header("<disabled>") {
            Err(Error {
                kind: Kind::Disabled,
                ..
            }) => (),
            Err(e) => Err(e)?,
            _ => unreachable!(),
        }

        match header("disabled backtrace") {
            Err(Error {
                kind: Kind::Disabled,
                ..
            }) => (),
            Err(e) => Err(e)?,
            _ => unreachable!(),
        }

        match header("unsupported backtrace") {
            Err(Error {
                kind: Kind::Unsupported,
                ..
            }) => (),
            Err(e) => Err(e)?,
            _ => unreachable!(),
        }

        match header("bunch of random garbage") {
            Err(Error {
                kind: Kind::UnexpectedInput(input),
                ..
            }) => assert_eq!("bunch of random garbage", input),
            Err(e) => Err(e)?,
            _ => unreachable!(),
        }

        Ok(())
    }
}
