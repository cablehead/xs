use crate::Error;
use std::collections::HashSet;
use ucd_parse::Codepoints::{Range, Single};
use ucd_parse::{Codepoint, CodepointRange, Codepoints};

pub fn add_codepoints(range: &CodepointRange, vec: &mut Vec<Codepoints>) {
    if range.start == range.end {
        vec.push(Single(range.start));
    } else {
        vec.push(Range(*range));
    }
}

pub fn insert_codepoint(cp: u32, set: &mut HashSet<u32>) -> Result<(), Error> {
    set.insert(cp)
        .then_some(())
        .ok_or_else(|| Error::from(format!("Codepoint already processed {:#06x}", cp).as_str()))
}

pub fn insert_codepoint_range(range: &CodepointRange, set: &mut HashSet<u32>) -> Result<(), Error> {
    for cp in range.start.value()..=range.end.value() {
        insert_codepoint(cp, set)?;
    }
    Ok(())
}

fn add_range(range: &Option<CodepointRange>, out: &mut Vec<Codepoints>) {
    if let Some(r) = range {
        if r.start == r.end {
            // Add single code point
            out.push(Single(r.start));
        } else {
            // Add range
            out.push(Range(*r));
        }
    };
}

pub fn get_codepoints_vector(codepoints: &HashSet<u32>) -> Vec<Codepoints> {
    let mut vec = Vec::new();
    codepoints.iter().for_each(|cp| {
        vec.push(cp);
    });
    vec.sort();

    let mut out = Vec::new();
    let mut range: Option<CodepointRange> = None;

    for cp in vec.iter() {
        match range.as_mut() {
            Some(r) => {
                if **cp - r.end.value() == 1 {
                    r.end = Codepoint::from_u32(**cp).unwrap();
                } else {
                    // there is a gap, non-consecutive numbers
                    add_range(&range, &mut out);
                    // Start a new range
                    range = Some(CodepointRange {
                        start: Codepoint::from_u32(**cp).unwrap(),
                        end: Codepoint::from_u32(**cp).unwrap(),
                    });
                }
            }
            None => {
                range = Some(CodepointRange {
                    start: Codepoint::from_u32(**cp).unwrap(),
                    end: Codepoint::from_u32(**cp).unwrap(),
                });
            }
        }
    }

    add_range(&range, &mut out);

    out
}
