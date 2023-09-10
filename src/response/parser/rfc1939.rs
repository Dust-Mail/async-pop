use bytes::Bytes;
use nom::{
    branch::alt,
    bytes::streaming::{tag, take_until, take_while, take_while_m_n},
    character::{
        is_alphanumeric,
        streaming::{char, digit1, not_line_ending, space0, space1},
    },
    combinator::{map, opt, value},
    multi::many_till,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

use crate::{
    constants::{ERR, OK},
    response::{
        list::List,
        stat::Stat,
        uidl::{Uidl, UniqueId},
        Response, Status,
    },
};

use super::core::{end_of_multiline, eol, message_parser};

pub(crate) fn status<'a>(input: &'a [u8]) -> IResult<&'a [u8], Status> {
    terminated(
        map(alt((value(true, tag(OK)), value(false, tag(ERR)))), |val| {
            Status::new(val)
        }),
        space0,
    )(input)
}

fn stat(input: &[u8]) -> IResult<&[u8], Stat> {
    let (input, count) = digit1(input)?;
    let (input, _) = space1(input)?;
    let (input, size) = digit1(input)?;
    let (input, _) = opt(not_line_ending)(input)?;
    let (input, _) = eol(input)?;

    Ok((input, Stat::new(count, size)))
}

pub(crate) fn stat_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, stats) = stat(input)?;

    Ok((input, Response::Stat(stats)))
}

fn list_stats(input: &[u8]) -> IResult<&[u8], Stat> {
    let (input, count) = digit1(input)?;
    let (input, _) = space1(input)?;
    let (input, _) = take_while(is_alphanumeric)(input)?;
    let (input, _) = space1(input)?;
    let (input, (size, _, _)) = delimited(
        char('('),
        tuple((digit1, space1, take_while(is_alphanumeric))),
        char(')'),
    )(input)?;

    let (input, _) = eol(input)?;

    let stats = Stat::new(count, size);

    Ok((input, stats))
}

pub(crate) fn list_response<'a>(input: &'a [u8]) -> IResult<&'a [u8], Response> {
    let (input, stats) = alt((
        map(list_stats, |stats| Some(stats)),
        map(message_parser, |_| None),
    ))(input)?;

    let (input, (items, _end)) = many_till(preceded(opt(tag(".")), stat), end_of_multiline)(input)?;

    let list = List::new(stats, items);

    Ok((input, Response::List(list)))
}

struct UniqueIdParser;

impl UniqueIdParser {
    fn is_valid_char(c: u8) -> bool {
        c >= 0x21 && c <= 0x7E
    }

    pub fn parse(input: &[u8]) -> IResult<&[u8], &[u8]> {
        take_while_m_n(1, 70, Self::is_valid_char)(input)
    }
}

fn uidl(input: &[u8]) -> IResult<&[u8], UniqueId> {
    let (input, index) = digit1(input)?;
    let (input, _) = space1(input)?;
    let (input, id) = UniqueIdParser::parse(input)?;
    let (input, _) = eol(input)?;

    Ok((input, UniqueId::new(index, id)))
}

pub(crate) fn uidl_list_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, message) = message_parser(input)?;

    let (input, (list, _end)) = many_till(preceded(opt(tag(".")), uidl), end_of_multiline)(input)?;

    let list = Uidl::new(message, list);

    Ok((input, Response::Uidl(list.into())))
}

pub(crate) fn uidl_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, unique_id) = uidl(input)?;

    Ok((input, Response::Uidl(unique_id.into())))
}

pub(crate) fn rfc822_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _message) = message_parser(input)?;

    let (input, content) = take_until("\r\n.\r\n")(input)?;

    let (input, _) = eol(input)?;
    let (input, _) = end_of_multiline(input)?;

    Ok((input, Response::Bytes(Bytes::copy_from_slice(content))))
}

pub(crate) fn error_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, message) = message_parser(input)?;

    let message = message.unwrap_or(b"");

    Ok((input, Response::Err(message.into())))
}

pub(crate) fn string_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, message) = message_parser(input)?;

    let message = message.unwrap_or(b"");

    Ok((input, Response::Message(message.into())))
}

#[cfg(test)]
mod test {
    use crate::response::types::DataType;

    use super::*;

    #[test]
    fn test_status() {
        let data = b"+OK\r\n";

        let (output, resp_status) = status(data).unwrap();

        assert!(output == b"\r\n");
        assert!(resp_status.success());

        let data = b"-ERR\r\n";

        let (output, resp_status) = status(data).unwrap();

        assert!(output == b"\r\n");
        assert!(!resp_status.success());
    }

    #[test]
    fn test_stat() {
        let data = b"1 120 bla bla\r\n";

        let (output, stats) = stat(data).unwrap();

        assert!(output.is_empty());
        assert!(stats.counter().value().unwrap() == 1);
        assert!(stats.size().value().unwrap() == 120);

        let data = b"1 sdf bla bla\r\n";

        let result = stat(data);

        assert!(result.is_err());
    }

    #[test]
    fn test_list_stats() {
        let data = b"2 messages (320 bytes)\r\n";

        let (input, stats) = list_stats(data).unwrap();

        assert!(input.is_empty());

        assert!(stats.counter().value().unwrap() == 2);
        assert!(stats.size().value().unwrap() == 320);

        let data = b"2 sdf%fg (320 sdf#$%fdg)\r\n";

        let result = list_stats(data);

        assert!(result.is_err());
    }

    #[test]
    fn test_rfc822() {
        let data = b"Date: Thu, 9 Sep 2023 15:30:00 -0400\r\nFrom: John Doe <johndoe@example.com>\r\nTo: Jane Smith <janesmith@example.com>\r\nSubject: Hello, Jane!\r\n\r\nDear Jane,\r\n\r\nI hope this message finds you well. I just wanted to say hello and see how you're doing.\r\n\r\nBest regards,\r\nJohn\r\n.\r\n";

        let (output, response) = rfc822_response(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Bytes(bytes) => {
                assert!(bytes.len() == 228)
            }
            _ => {
                unreachable!()
            }
        }
    }
}
