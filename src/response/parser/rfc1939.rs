use nom::{
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::{digit1, space1},
    combinator::{map, map_res, value},
    sequence::terminated,
    IResult,
};

use crate::{
    constants::{ERR, OK},
    response::{list::ListItem, stat::StatResponse, uidl::UniqueId, Status},
};

use super::eol;

pub(crate) fn status<'a>(input: &'a str) -> IResult<&'a str, Status> {
    terminated(
        map(alt((value(true, tag(OK)), value(false, tag(ERR)))), |val| {
            Status::new(val)
        }),
        space1,
    )(input)
}

pub(crate) fn stat(input: &str) -> IResult<&str, StatResponse> {
    let (input, count) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, size) = map_res(digit1, str::parse)(input)?;
    let (input, _) = eol(input)?;

    Ok((input, StatResponse::new(count, size)))
}

pub(crate) fn list(input: &str) -> IResult<&str, ListItem> {
    let (input, index) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, size) = map_res(digit1, str::parse)(input)?;
    let (input, _) = eol(input)?;

    Ok((input, ListItem::new(index, size)))
}

struct UniqueIdParser;

impl UniqueIdParser {
    fn is_valid_char(c: char) -> bool {
        (c as u32) >= 0x21 && (c as u32) <= 0x7E
    }

    pub fn parse(input: &str) -> IResult<&str, &str> {
        let (input, id) = take_while_m_n(1, 70, Self::is_valid_char)(input)?;
        Ok((input, id))
    }
}

pub(crate) fn uidl(input: &str) -> IResult<&str, UniqueId> {
    let (input, index) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, id) = UniqueIdParser::parse(input)?;
    let (input, _) = eol(input)?;

    Ok((input, UniqueId::new(index, id.into())))
}
