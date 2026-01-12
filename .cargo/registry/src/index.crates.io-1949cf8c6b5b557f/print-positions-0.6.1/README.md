![Build and test](https://github.com/bobhy/print-positions/actions/workflows/ci.yml/badge.svg)
# Crate print_positions
Iterators which return the slice of characters making up a "print position", rather 
than the individual characters of a source string.  

* [Documentation](https://docs.rs/print-positions)
* [crates.io](https://crates.io/crates/print-positions)
* [Release notes](https://github.com/bobhy/print-positions/blob/main/CHANGELOG.md)

 The [print_positions](https://docs.rs/print-positions/latest/print_positions/fn.print_positions.html) 
 and [print_position_indices](https://docs.rs/print-positions/latest/print_positions/fn.print_position_indices.html) functions 
 provide iterators which return "print positions".

 A print position is a generalization of a
 [UAX#29 extended grapheme cluster](http://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries).
 Like the grapheme, it occupies one "character" when rendered on the screen.  
 However, it may also contain [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code#Description) 
 which affect color or intensity rendering as well.

 ## Example:
 ```rust
 use print_positions::print_positions;

 // content is e with dieresis, displayed in green with a color reset at the end.  
 // Looks like 1 character on the screen.  See example "padding" to print one out.
 let content = ["\u{1b}[30;42m", "\u{0065}", "\u{0308}", "\u{1b}[0m"].join("");
 
 let print_positions:Vec<_> = print_positions(&content).collect();
 assert_eq!(content.len(), 15);          // content is 15 chars long
 assert_eq!(print_positions.len(), 1);   // but only 1 print position
 ```
 ## Rationale:
 When laying out a fixed-width screen application, it is useful to know how many visible 
 columns a piece of content will consume.  But the number of bytes or characters in
 the content is generally larger, inflated by UTF8 encoding, Unicode combining characters 
 and zero-width joiners and, for ANSI compatible devices and applications, by control codes and escape
 sequences which specify text color and emphasis.  
 
 The print_position iterators account for these factors
 and simplify the arithmetic: the number of columns the content will consume on the screen is 
 the number of print position slices returned by the iterator.
## Known Issues:
* No accounting for cursor motion  
ANSI control characters and sequences are *all* assumed to consume no space on the screen.   
This is arguably a bug 
in the case of backspace, tab, newline, CUP, CUU, CUD and several more.  PRs or simple suggestions for improvement are welcome!
