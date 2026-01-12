#![no_implicit_prelude]
#![no_std]
#![cfg_attr(feature = "step_trait", feature(step_trait))]
#![cfg(feature = "macro")]
#![forbid(clippy::pedantic)]

#[allow(dead_code, non_camel_case_types)]
mod primitives {
    struct u8 {}
    struct u16 {}
    struct u32 {}
    struct u64 {}
    struct u128 {}
    struct usize {}
    struct i8 {}
    struct i16 {}
    struct i32 {}
    struct i64 {}
    struct i128 {}
    struct isize {}
}

::bounded_integer::bounded_integer! {
    #[repr(isize)]
    pub struct StructSigned { -3..2 }
}
::bounded_integer::bounded_integer! {
    #[repr(u16)]
    pub struct StructUnsigned { 36..65535 }
}
::bounded_integer::bounded_integer! {
    #[repr(i64)]
    pub enum EnumSigned { -4..6 }
}
::bounded_integer::bounded_integer! {
    #[repr(u8)]
    pub enum EnumUnsigned { 253..255 }
}
