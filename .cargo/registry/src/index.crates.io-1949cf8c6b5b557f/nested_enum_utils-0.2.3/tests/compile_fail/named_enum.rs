use nested_enum_utils::enum_conversions;

#[derive(Debug)]
#[enum_conversions(Outer)]
enum Enum {
    A { value : u8 },
}

fn main() {}
