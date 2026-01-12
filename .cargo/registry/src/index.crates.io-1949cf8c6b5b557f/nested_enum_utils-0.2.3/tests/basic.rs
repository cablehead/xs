use nested_enum_utils::{common_fields, enum_conversions};

#[test]
fn test_single_enum() {
    #[derive(Debug)]
    #[enum_conversions]
    enum Test {
        A(u32),
        B(String),
    }

    // convert from leaf to enum
    let e: Test = 42u32.into();
    // convert from enum to leaf by reference
    let lr: &u32 = (&e).try_into().unwrap();
    assert_eq!(*lr, 42);
    // convert from enum to leaf by value
    let l: u32 = e.try_into().unwrap();
    assert_eq!(l, 42);
}

#[test]
fn test_nested_enums() {
    #[derive(Debug)]
    #[enum_conversions(Outer)]
    enum Inner {
        A(u32),
        B(u8),
    }

    #[derive(Debug)]
    #[enum_conversions]
    enum Outer {
        A(Inner),
        B(String),
    }

    // convert from leaf to outer
    let e: Outer = 42u32.into();
    // convert from outer to leaf by reference
    let lr: &u32 = (&e).try_into().unwrap();
    assert_eq!(*lr, 42);
    // convert from outer to leaf by value
    let l: u32 = e.try_into().unwrap();
    assert_eq!(l, 42);
}

#[test]
fn test_deeply_nested_enums() {
    #[derive(Debug)]
    #[enum_conversions(Outer)]
    enum Inner {
        A(u32),
        B(u8),
    }

    #[derive(Debug)]
    #[enum_conversions(Outer)]
    enum Mid {
        A(Inner),
        B(String),
    }

    #[derive(Debug)]
    #[enum_conversions]
    enum Outer {
        A(Mid),
        B(f32),
    }

    // convert from leaf to outer
    let e: Outer = 42u32.into();
    // convert from outer to leaf by reference
    let lr: &u32 = (&e).try_into().unwrap();
    assert_eq!(*lr, 42);
    // convert from outer to leaf by value
    let l: u32 = e.try_into().unwrap();
    assert_eq!(l, 42);
}

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

#[test]
fn test_common_fields() {
    #[common_fields({ id: u64 })]
    #[allow(dead_code)]
    enum Test {
        A { x: u32 },
        B { y: String },
    }
    let _v = Test::A { x: 42, id: 1 };
}
