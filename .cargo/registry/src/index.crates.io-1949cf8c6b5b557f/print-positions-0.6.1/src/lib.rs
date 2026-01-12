//! The [print_positions] and [print_position_data] functions
//! provide iterators which return "print positions".
//!
//! A print position is a generalization of the
//! [UAX#29 extended grapheme cluster](http://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries) to include rendering color and emphasis of the user-visible
//! character using 
//! [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code#Description).
//! So a "print position" is an even longer multi-byte sequence that still represents a single user visible character on the screen.
//!
//! ## Example:
//! ```rust
//! use print_positions::{print_positions, print_position_data};
//!
//! // content is e with dieresis, displayed in green with a color reset at the end.  
//! // Looks like 1 character on the screen.  See example "padding" to print one out.
//! let content = &["\u{1b}[30;42m", "\u{0065}", "\u{0308}", "\u{1b}[0m"].join("");
//!
//! // access number of print positions without examining the content
//! assert_eq!(print_positions(content).count(), 1);
//! 
//! let segmented:Vec<_> = print_position_data(content).collect();
//! assert_eq!(content.len(), 15);          // content is 15 chars long
//! assert_eq!(segmented.len(), 1);   // but only 1 print position
//! 
//! ```
//! ## Rationale:
//! In the good old days, a "character" was a simple entity.  It would always fit into one octet 
//! (or perhaps only a [sestet](https://retrocomputing.stackexchange.com/questions/7937/last-computer-not-to-use-octets-8-bit-bytes)).
//! You could access the i'th character in a string by accessing the i'th element of its array.  
//! And you could process characters in any human language you wanted, as long as it was (transliterated into) English.
//! 
//! Modern applications must support multiple natural languages and some are rendered on an ANSI-compatible
//! screen (or, less often, print device). So it's a given that what a user would consider a simple "character", visible as a single
//! glyph on the screen, is represented in memory by multiple and variable numbers of bytes.
//! 
//! This crate provides a tool to make it once again easy to access the i'th "character" of a word on the screen
//! by indexing to the i'th element of an array, but the array now consists of "print positions" rather than bytes or primitive type `char`s. 
//! See iterator [PrintPositionData].
//! 
//! Sometimes you don't even need to access the character data itself, you just want to know how many visible
//! columns it will consume on the screen, in order to align it with other text or within a fixed area on the screen.  See iterator [PrintPositions].
//!

#[cfg(test)]
mod tests;

use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

/// This iterator identifies print positions in the source string and returns start and end offsets of 
/// the data rather than the data itself.
/// See [PrintPositionData] if you want to iterate through the data instead.
/// 
/// A print position is an immutable slice of the source string.  It contains 1 grapheme cluster (by definition)
/// and any ANSI escape codes found between graphemes in the source.  The ANSI escape codes will generally *preceed*
/// the grapheme (since these codes change the rendering of characters that follow), but sometimes will *follow* the
/// grapheme (for the few codes that reset special graphic rendering).
/// 
/// ```rust
/// use print_positions::print_positions;
///
/// let content = "\u{1f468}\u{200d}\u{1f467}\u{200d}\u{1f466}abc";
/// let segments: Vec<(usize, usize)> = print_positions(content).collect();
/// assert_eq!(vec!((0, 18), (18, 19), (19, 20), (20, 21)), segments);
/// 
/// // access print position data after segmenting source.
/// assert_eq!( &content[segments[1].0..segments[1].1], "a"); 
/// 
/// // Count print positions in content.
/// assert_eq!( print_positions(content).count(), 4);
/// ```
#[derive(Clone)]
pub struct PrintPositions<'a> {
    // the victim string -- all outputs are slices of this.
    string: &'a str,
    // offset of beginning of slice currently being assembled or last returned.
    cur_offset: usize,
    // offset of the first unexamined char
    next_offset: usize,
    // wrapped grapheme (== extended grapheme cluster) iterator
    gi_iterator: GraphemeIndices<'a>,
}
/// Factory method to create a new [PrintPositions] iterator
///
#[inline]
pub fn print_positions<'a>(s: &'a str) -> PrintPositions<'a> {
    let iter = UnicodeSegmentation::grapheme_indices(s, true);
    PrintPositions {
        string: s,
        cur_offset: 0,
        next_offset: 0,
        gi_iterator: iter,
    }
}

impl<'a> PrintPositions<'a> {
    /// View the underlying data (the part yet to be iterated) as a slice of the original string.
    ///
    /// ```rust
    /// # use print_positions::print_positions;
    /// let mut iter = print_positions("abc");
    /// assert_eq!(iter.as_str(), "abc");
    /// iter.next();
    /// assert_eq!(iter.as_str(), "bc");
    /// iter.next();
    /// iter.next();
    /// assert_eq!(iter.as_str(), "");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &'a str {
        &self.string[self.cur_offset..self.string.len()]
    }
}

impl<'a> Iterator for PrintPositions<'a> {
    /// Iterator returns tuple of start offset and end + 1 offset
    /// in source string of current print position.
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset > self.string.len() {
            return None;
        };

        enum EscapeState {
            Normal,
            EscapeSeen, // just saw an escape, start accumulating
            CSISeen,    // 2nd char not terminal, continue accumulating
            OSCSeen,    // operating system commmand, accumulate through ESC\.
            OSCSeen1,   // in OSC, saw ESC, look for \
        }

        let mut escape_state = EscapeState::Normal;

        while self.next_offset < self.string.len() {
            let grap = self.gi_iterator.next().expect("already checked not at EOS");
            debug_assert_eq!(
                grap.0, self.next_offset,
                "offset of retrieved grap (left) not at start of rest of string (right)",
            );
            self.next_offset += grap.1.len();

            let ascii_byte = grap.1.as_bytes()[0];

            match escape_state {
                EscapeState::Normal => {
                    if ascii_byte == 0x1b {
                        escape_state = EscapeState::EscapeSeen;
                    } else {
                        break; // terminate the grapheme
                    }
                }

                EscapeState::EscapeSeen => match ascii_byte {
                    b'[' => {
                        escape_state = EscapeState::CSISeen;
                    }
                    b']' => {
                        escape_state = EscapeState::OSCSeen;
                    }
                    0x40..=0x5F => {
                        // terminate escape, but continue accumulating rest of print position
                        escape_state = EscapeState::Normal;
                    }
                    _ => {
                        debug_assert!(
                            true, // don't actually fail fuzz testing, but document behavior for malformed escapes.
                            "unexpected char {ascii_byte} following ESC, terminating escape"
                        );
                        escape_state = EscapeState::Normal;
                    }
                },

                EscapeState::CSISeen => {
                    if (0x40..=0x7e).contains(&ascii_byte) {
                        // end of CSI, but continue accumulating
                        escape_state = EscapeState::Normal;
                    } else if (0x20..=0x3f).contains(&ascii_byte) { // accumulate CSI
                    } else {
                        debug_assert!(
                            true, // don't actually fail fuzz testing, but document behavior for malformed escapes.
                            "unexpected char {ascii_byte} in CSI sequence, terminating escape"
                        );
                        escape_state = EscapeState::Normal;
                    }
                }

                EscapeState::OSCSeen => {
                    if ascii_byte == 0x07 {
                        // spec says BEL terminates seq (on some emulators)
                        escape_state = EscapeState::Normal;
                    } else if ascii_byte == 0x1b {
                        escape_state = EscapeState::OSCSeen1;
                    } // anything else stays in OSC accumulation
                }

                EscapeState::OSCSeen1 => {
                    match ascii_byte {
                        0x5c => {
                            // backslash
                            escape_state = EscapeState::Normal;
                        }
                        0x1b => {
                            escape_state = EscapeState::OSCSeen1;
                        }
                        _ => {
                            escape_state = EscapeState::OSCSeen;
                        }
                    }
                }
            }
        }

        // before returning, peek ahead and see whether there's a reset escape sequence we can append.
        // There are 3 ANSI reset sequences.
        // if, perversely, there is more than one sequence following the grapheme, take them all.
        // If, even more perversely, the last char of the esc sequence plus some following
        // characters in the string happen to form a multi-character grapheme, take all of that.
        // This means that the reset escape sequence is not always the end of the print position slice.

        while self.next_offset < self.string.len()
            && self.string.as_bytes()[self.next_offset] == 0x1b
        {
            if self.next_offset + 2 <= self.string.len()
                && self.string[self.next_offset..].starts_with("\x1bc")
            {
                self.gi_iterator.next();
                let last = self.gi_iterator.next().expect("must be >=2");
                self.next_offset += 1 + last.1.len();
            } else if self.next_offset + 3 <= self.string.len()
                && self.string[self.next_offset..].starts_with("\x1b[m")
            {
                self.gi_iterator.next();
                self.gi_iterator.next();
                let last = self.gi_iterator.next().expect("must be >=3");
                self.next_offset += 2 + last.1.len();
            } else if self.next_offset + 4 <= self.string.len()
                && self.string[self.next_offset..].starts_with("\x1b[0m")
            {
                self.gi_iterator.next();
                self.gi_iterator.next();
                self.gi_iterator.next();
                let last = self.gi_iterator.next().expect("must be >=4");
                self.next_offset += 3 + last.1.len();
            } else {
                break; // ESC then something else.  Take it at the beginning of the next call.
            }
        }
        // return everything between start and end offsets
        if self.next_offset <= self.cur_offset {
            return None;
        } else {
            let retval = (self.cur_offset, self.next_offset);
            // advance start to one beyond end of what we're returning
            self.cur_offset = self.next_offset;
            return Some(retval);
        }
    }
}


/// This iterator returns "print position" data found in a string, as an immutable slice within the source string.  
/// 
/// All the source bytes are passed through the iterator in order and without modification, except they are grouped or "segmented" into print position slices.
/// 
/// ```rust
/// use print_positions::print_position_data;
///
/// let segs: Vec<_> = print_position_data("abc\u{1f468}\u{200d}\u{1f467}\u{200d}\u{1f466}").collect();
/// assert_eq!(vec!("a","b","c",
///     "\u{1f468}\u{200d}\u{1f467}\u{200d}\u{1f466}"   // unicode family emoji -- 1 print position
///     ), segs);
///
/// // Control chars and ANSI escapes returned within the print position slice.
/// let content = "abc\u{1b}[37;46mdef\u{1b}[0mg";
/// let segs: Vec<_> = print_position_data(content).collect();
/// assert_eq!(vec!("a","b","c", "\u{1b}[37;46md","e","f\u{1b}[0m", "g"), segs);
/// assert_eq!(content, segs.join(""), "all characters passed through iterator transparently");
/// ```
///
/// Run `cargo run --example padding`
/// for an example of fixed-width formatting based on counting print positions
/// rather than characters in the data.
///
pub struct PrintPositionData<'a>(PrintPositions<'a>);

#[inline]
/// Factory method to provide a new [PrintPositionData] iterator.
///
pub fn print_position_data<'a>(s: &'a str) -> PrintPositionData<'a> {
    PrintPositionData(print_positions(s))
}

impl<'a> PrintPositionData<'a> {
    /// View the underlying data (the part yet to be iterated) as a slice of the original string.
    ///
    /// ```rust
    /// # use print_positions::print_position_data;
    /// let mut iter = print_position_data("abc");
    /// assert_eq!(iter.as_str(), "abc");
    /// iter.next();
    /// assert_eq!(iter.as_str(), "bc");
    /// iter.next();
    /// iter.next();
    /// assert_eq!(iter.as_str(), "");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &'a str {
        &self.0.string[self.0.cur_offset..self.0.string.len()]
    }
}

impl<'a> Iterator for PrintPositionData<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((start, end)) = self.0.next() {
            Some(&self.0.string[start..end])
        } else {
            None
        }
    }
}

