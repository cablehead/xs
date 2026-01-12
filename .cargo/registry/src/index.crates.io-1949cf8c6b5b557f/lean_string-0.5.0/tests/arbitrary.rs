// from: https://github.com/ParkMyCar/compact_str/blob/193d13eaa5a92b3c39c2f7289dc44c95f37c80d1/compact_str/src/features/arbitrary.rs
#![cfg(feature = "arbitrary")]

use arbitrary::{Arbitrary, Unstructured};
use lean_string::LeanString;

#[test]
fn arbitrary_sanity() {
    let mut data = Unstructured::new(&[42; 50]);
    let compact = LeanString::arbitrary(&mut data).expect("generate a CompactString");

    // we don't really care what the content of the CompactString is, just that one's generated
    assert!(!compact.is_empty());
}

#[test]
fn arbitrary_inlines_strings() {
    let mut data = Unstructured::new(&[42; 8]);
    let compact = LeanString::arbitrary(&mut data).expect("generate a CompactString");

    // running this manually, we generate the string "**"
    assert!(!compact.is_heap_allocated());
}
