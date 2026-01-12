use nested_enum_utils::common_fields;

#[common_fields({ x: u64 })]
enum Enum {
    A,
    B {},
}

fn main() {}
