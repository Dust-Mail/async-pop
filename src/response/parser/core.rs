use nom::{
    bytes::complete::tag,
    character::complete::{line_ending, not_line_ending, space0},
    combinator::opt,
    sequence::{pair, terminated},
    IResult,
};

pub fn eol(input: &str) -> IResult<&str, ()> {
    let (input, _) = pair(space0, line_ending)(input)?;

    Ok((input, ()))
}

pub fn end_of_multiline(input: &str) -> IResult<&str, ()> {
    let (input, _) = pair(tag("."), line_ending)(input)?;

    Ok((input, ()))
}

pub fn message_parser<'a>(input: &'a str) -> IResult<&'a str, Option<&'a str>> {
    terminated(opt(not_line_ending), eol)(input)
}
