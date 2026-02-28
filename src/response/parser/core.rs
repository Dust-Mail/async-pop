use nom::{
    bytes::streaming::tag,
    character::streaming::{line_ending, not_line_ending, space0},
    combinator::opt,
    sequence::{pair, terminated},
    IResult,
};

pub fn eol(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = pair(space0, line_ending)(input)?;

    Ok((input, ()))
}

pub fn end_of_multiline(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = pair(tag(b"."), line_ending)(input)?;

    Ok((input, ()))
}

pub fn message_parser(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    terminated(opt(not_line_ending), eol)(input)
}
