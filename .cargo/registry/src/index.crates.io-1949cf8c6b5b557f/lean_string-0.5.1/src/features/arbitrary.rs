use crate::LeanString;
use arbitrary::{Arbitrary, Result, Unstructured};

#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
impl<'a> Arbitrary<'a> for LeanString {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        <&str as Arbitrary>::arbitrary(u).map(LeanString::from)
    }

    fn arbitrary_take_rest(u: Unstructured<'a>) -> Result<Self> {
        <&str as Arbitrary>::arbitrary_take_rest(u).map(LeanString::from)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <&str as Arbitrary>::size_hint(depth)
    }
}
