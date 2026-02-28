use nom::{bytes::streaming::tag, character::streaming::space1, IResult};

use super::core::message_parser;

pub(crate) fn auth(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = tag("+")(input)?;
    let (input, _) = space1(input)?;
    let (input, content) = message_parser(input)?;

    Ok((input, content.unwrap_or(b"")))
}
