use bytes::Bytes;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while_m_n},
    character::complete::{digit1, not_line_ending, space0, space1},
    combinator::{map, map_res, opt, value},
    multi::many_till,
    sequence::terminated,
    IResult,
};

use crate::{
    constants::{ERR, OK},
    response::{
        list::{List, ListItem},
        stat::StatResponse,
        uidl::{Uidl, UniqueId},
        Response, Status,
    },
};

use super::core::{end_of_multiline, eol, message_parser};

pub(crate) fn status<'a>(input: &'a str) -> IResult<&'a str, Status> {
    terminated(
        map(alt((value(true, tag(OK)), value(false, tag(ERR)))), |val| {
            Status::new(val)
        }),
        space0,
    )(input)
}

fn stat(input: &str) -> IResult<&str, StatResponse> {
    let (input, count) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, size) = map_res(digit1, str::parse)(input)?;
    let (input, _) = opt(not_line_ending)(input)?;
    let (input, _) = eol(input)?;

    Ok((input, StatResponse::new(count, size)))
}

pub(crate) fn stat_response(input: &str) -> IResult<&str, Response> {
    let (input, stats) = stat(input)?;

    Ok((input, Response::Stat(stats)))
}

fn list(input: &str) -> IResult<&str, ListItem> {
    let (input, index) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, size) = map_res(digit1, str::parse)(input)?;
    let (input, _) = opt(not_line_ending)(input)?;
    let (input, _) = eol(input)?;

    Ok((input, ListItem::new(index, size)))
}

pub(crate) fn list_response(input: &str) -> IResult<&str, Response> {
    let (input, message) = message_parser(input)?;

    let (input, (items, _end)) = many_till(list, end_of_multiline)(input)?;

    let list = List::new(message, items);

    Ok((input, Response::List(list)))
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

fn uidl(input: &str) -> IResult<&str, UniqueId> {
    let (input, index) = map_res(digit1, str::parse)(input)?;
    let (input, _) = space1(input)?;
    let (input, id) = UniqueIdParser::parse(input)?;
    let (input, _) = eol(input)?;

    Ok((input, UniqueId::new(index, id.into())))
}

pub(crate) fn uidl_list_response(input: &str) -> IResult<&str, Response> {
    let (input, message) = message_parser(input)?;

    let (input, (list, _end)) = many_till(uidl, end_of_multiline)(input)?;

    let list = Uidl::new(message, list);

    Ok((input, Response::Uidl(list.into())))
}

pub(crate) fn uidl_response(input: &str) -> IResult<&str, Response> {
    let (input, unique_id) = uidl(input)?;

    Ok((input, Response::Uidl(unique_id.into())))
}

pub(crate) fn rfc822_response(input: &str) -> IResult<&str, Response> {
    let (input, _message) = message_parser(input)?;

    let (input, content) = take_until(".\r\n")(input)?;

    let (input, _) = end_of_multiline(input)?;

    let bytes = Bytes::from(content.to_string());

    Ok((input, Response::Bytes(bytes)))
}

pub(crate) fn error_response(input: &str) -> IResult<&str, Response> {
    let (input, message) = message_parser(input)?;

    let message = message.unwrap_or("");

    Ok((input, Response::Err(message.to_string())))
}

pub(crate) fn string_response(input: &str) -> IResult<&str, Response> {
    let (input, message) = message_parser(input)?;

    let message = message.unwrap_or("");

    Ok((input, Response::Message(message.to_string())))
}
