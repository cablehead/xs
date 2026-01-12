use nom::{
    character::complete::digit1,
    combinator::{map_res, opt},
    IResult,
};

pub(crate) fn parse_u8(input: &str) -> IResult<&str, u8> {
    map_res(digit1, decimal_u8)(input)
}

pub(crate) fn decimal_u8(input: &str) -> Result<u8, core::num::ParseIntError> {
    input.parse::<u8>()
}

pub(crate) fn parse_u32_default(input: &str, default: u32) -> IResult<&str, u32> {
    parse_u32(input).map(|(input, n)| (input, n.unwrap_or(default)))
}

pub(crate) fn parse_u32(input: &str) -> IResult<&str, Option<u32>> {
    opt(map_res(digit1, decimal_u32))(input)
}

pub(crate) fn decimal_u32(input: &str) -> Result<u32, core::num::ParseIntError> {
    input.parse::<u32>()
}
