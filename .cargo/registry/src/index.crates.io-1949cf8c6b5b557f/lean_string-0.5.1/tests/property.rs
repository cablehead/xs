use lean_string::{LeanString, ToLeanString};
use proptest::{prelude::*, property_test};

#[property_test]
#[cfg_attr(miri, ignore)]
fn create_from_str(input: String) {
    let str = input.as_str();

    let lean = LeanString::from(str);
    prop_assert_eq!(&lean, str);
    prop_assert_eq!(lean.len(), str.len());

    if str.len() <= 2 * size_of::<usize>() {
        prop_assert!(!lean.is_heap_allocated());
    } else {
        prop_assert!(lean.is_heap_allocated());
    }
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn create_from_u8_bytes(input: Vec<u8>) {
    let bytes = input.as_slice();

    let lean = LeanString::from_utf8(bytes);
    let string = String::from_utf8(bytes.to_vec());
    prop_assert_eq!(lean.is_err(), string.is_err());
    if let (Ok(lean), Ok(string)) = (lean, string) {
        prop_assert_eq!(&lean, &string);
    }

    let lean = LeanString::from_utf8_lossy(bytes);
    let string = String::from_utf8_lossy(bytes);
    prop_assert_eq!(&lean, &string);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn create_from_u16_bytes(input: Vec<u16>) {
    let bytes = input.as_slice();

    let lean = LeanString::from_utf16(bytes);
    let string = String::from_utf16(bytes);
    prop_assert_eq!(lean.is_err(), string.is_err());
    if let (Ok(lean), Ok(string)) = (lean, string) {
        prop_assert_eq!(&lean, &string);
    }

    let lean = LeanString::from_utf16_lossy(bytes);
    let string = String::from_utf16_lossy(bytes);
    prop_assert_eq!(&lean, &string);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn collect_from_chars(input: String) {
    let lean = input.chars().collect::<LeanString>();
    prop_assert_eq!(&lean, &input);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn collect_from_strings(input: Vec<String>) {
    let lean = input.clone().into_iter().collect::<LeanString>();
    let string = input.into_iter().collect::<String>();
    prop_assert_eq!(&lean, &string);
}

macro_rules! test_integer_to_lean_string {
    ($($ty:ty),* $(,)?) => {$(
        paste::paste! {
            #[test]
            fn [<$ty _to_lean_string>]() {
                for num in <$ty>::MIN..=<$ty>::MAX {
                    let lean = num.to_lean_string();
                    let string = num.to_string();
                    assert_eq!(lean, string);
                }
            }
            #[test]
            fn [<nonzero_ $ty _to_lean_string>]() {
                for num in <$ty>::MIN..=<$ty>::MAX {
                    if num == 0 { continue };
                    let num = core::num::NonZero::<$ty>::new(num).unwrap();
                    let lean = num.to_lean_string();
                    let string = num.to_string();
                    assert_eq!(lean, string);
                }
            }
        }
    )*};
}
test_integer_to_lean_string!(u8, i8);

macro_rules! prop_test_integer_to_lean_string {
    ($($ty:ty),* $(,)?) => {$(
        paste::paste! {
            #[property_test]
            #[cfg_attr(miri, ignore)]
            fn [<$ty _to_lean_string>](i: $ty) {
                prop_assert_eq!(i.to_lean_string(), i.to_string());
            }
            #[property_test]
            #[cfg_attr(miri, ignore)]
            fn [<nonzero_ $ty _to_lean_string>](i: core::num::NonZero<$ty>) {
                prop_assert_eq!(i.to_lean_string(), i.to_string());
            }
        }
    )*};
}
prop_test_integer_to_lean_string!(u16, i16, u32, i32, u64, i64, u128, i128, usize, isize);

#[property_test]
#[cfg_attr(miri, ignore)]
fn f32_to_lean_string(f: f32) {
    let lean = f.to_lean_string();
    let float = lean.parse::<f32>().unwrap();
    prop_assert_eq!(f, float);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn f64_to_lean_string(f: f64) {
    let lean = f.to_lean_string();
    let float = lean.parse::<f64>().unwrap();
    prop_assert_eq!(f, float);
}

#[test]
fn bool_to_lean_string() {
    let t = true;
    let f = false;
    assert_eq!(t.to_lean_string(), t.to_string());
    assert_eq!(f.to_lean_string(), f.to_string());
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn char_to_lean_string(c: char) {
    prop_assert_eq!(c.to_lean_string(), c.to_string());
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn string_to_lean_string(s: String) {
    prop_assert_eq!(s.to_lean_string(), s);
}
