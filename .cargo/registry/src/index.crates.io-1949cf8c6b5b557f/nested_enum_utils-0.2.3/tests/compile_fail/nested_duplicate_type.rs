use nested_enum_utils::enum_conversions;

#[derive(Debug)]
#[enum_conversions(Outer)]
enum Inner1 {
    A(u32),
    B(u8),
}

#[derive(Debug)]
#[enum_conversions(Outer)]
enum Inner2 {
    A(u32),
    B(u8),
}

#[derive(Debug)]
#[enum_conversions]
enum Outer {
    A(Inner1),
    B(Inner2),
}

fn main() {}
