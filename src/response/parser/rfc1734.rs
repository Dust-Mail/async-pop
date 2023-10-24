use nom::{bytes::streaming::tag, character::streaming::space1, IResult};

use super::core::message_parser;

pub(crate) fn auth<'a>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
    let (input, _) = tag("+")(input)?;
    let (input, _) = space1(input)?;
    let (input, content) = message_parser(input)?;

    Ok((input, content.unwrap_or(b"")))
}
