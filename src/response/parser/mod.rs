mod rfc1939;
mod rfc2449;

use bytes::Bytes;
use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{line_ending, not_line_ending, space0},
    combinator::opt,
    multi::many_till,
    sequence::{pair, terminated},
    IResult,
};

use crate::command::Command;

use self::{
    rfc1939::{list, stat, status, uidl},
    rfc2449::capability_response,
};

use super::{list::List, uidl::Uidl, Response, ResponseType};

fn eol(input: &str) -> IResult<&str, ()> {
    let (input, _) = pair(space0, line_ending)(input)?;

    Ok((input, ()))
}

fn end_of_multiline(input: &str) -> IResult<&str, ()> {
    let (input, _) = pair(tag("."), line_ending)(input)?;

    Ok((input, ()))
}

fn message_parser<'a>(input: &'a str) -> IResult<&'a str, Option<&'a str>> {
    terminated(opt(not_line_ending), eol)(input)
}

pub struct ResponseParser(Command);

impl ResponseParser {
    pub fn new(command: Command) -> Self {
        ResponseParser(command)
    }
}

impl ResponseParser {
    pub fn r#type<'a>(&'a self, input: &'a str) -> IResult<&'a str, ResponseType> {
        match self.0 {
            Command::Stat => {
                let (input, stats) = stat(input)?;

                Ok((input, ResponseType::Stat(stats)))
            }
            Command::List => {
                if input.lines().count() > 1 {
                    let (input, message) = message_parser(input)?;

                    let (input, (list, _end)) = many_till(list, end_of_multiline)(input)?;

                    let list = List::new(message, list);

                    Ok((input, ResponseType::List(list.into())))
                } else {
                    let (input, list_item) = list(input)?;

                    Ok((input, ResponseType::List(list_item.into())))
                }
            }
            Command::Retr => {
                let (input, _message) = message_parser(input)?;

                let (input, content) = take_until(".\r\n")(input)?;

                let bytes = Bytes::from(content.to_string());

                Ok((input, ResponseType::Retr(bytes)))
            }
            Command::Noop => Ok((input, ResponseType::Noop)),
            Command::Top => {
                let (input, _message) = message_parser(input)?;

                let (input, content) = take_until(".\r\n")(input)?;

                let bytes = Bytes::from(content.to_string());

                Ok((input, ResponseType::Top(bytes)))
            }
            Command::Uidl => {
                if input.lines().count() > 1 {
                    let (input, message) = message_parser(input)?;

                    let (input, (list, _end)) = many_till(uidl, end_of_multiline)(input)?;

                    let list = Uidl::new(message, list);

                    Ok((input, ResponseType::Uidl(list.into())))
                } else {
                    let (input, unique_id) = uidl(input)?;

                    Ok((input, ResponseType::Uidl(unique_id.into())))
                }
            }
            Command::Capa => capability_response(input),
            _ => {
                let (input, message) = message_parser(input)?;

                let message = message.unwrap_or("");

                Ok((input, ResponseType::Message(message.into())))
            }
        }
    }

    pub fn parse<'a>(&'a self, input: &'a str) -> IResult<&'a str, Response> {
        let (input, status) = status(input)?;

        let (input, r#type) = if !status.success() {
            (input, ResponseType::Err(input.into()))
        } else {
            self.r#type(input)?
        };

        Ok((input, Response::new(status, r#type)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{command::Command::*, response::list::ListResponse};

    #[test]
    fn test_status() {
        let data = "+OK ";

        let (output, resp_status) = status(data).unwrap();

        assert!(output.is_empty());
        assert!(resp_status.success());

        let data = "-ERR ";

        let (output, resp_status) = status(data).unwrap();

        assert!(output.is_empty());
        assert!(!resp_status.success());

        let data = "-ERR";

        let result = status(data);

        assert!(result.is_err());
    }

    #[test]
    fn test_list() {
        let parser = ResponseParser::new(List);

        let data = "+OK List follows:\r\n1 120\r\n2 200\r\n.\r\n";

        let (output, list) = parser.parse(data).unwrap();

        assert!(output.is_empty());

        match list.body() {
            ResponseType::List(list) => match list {
                ListResponse::Multiple(list) => {
                    assert!(list.items().len() == 2);
                    assert!(list.message().unwrap() == "List follows:")
                }
                _ => {
                    unreachable!()
                }
            },
            _ => {
                unreachable!()
            }
        }

        let data = "+OK List follows:\r\n1 120\r\n2 200\r\n";

        let result = parser.parse(data);

        assert!(result.is_err());

        let data = "+OK 1 120\r\n";

        let (output, response) = parser.parse(data).unwrap();

        assert!(output.is_empty());

        match response.body() {
            ResponseType::List(list) => match list {
                ListResponse::Single(item) => {
                    assert!(item.index() == 1 && item.size() == 120)
                }
                _ => {
                    unreachable!()
                }
            },
            _ => {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_stat() {
        let parser = ResponseParser::new(Stat);

        let data = "+OK 20 600\r\n";

        let (output, response) = parser.parse(data).unwrap();

        assert!(output.is_empty());

        match response.body() {
            ResponseType::Stat(stat) => {
                assert!(stat.message_count() == 20);
                assert!(stat.size() == 600);
            }
            _ => {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_capa() {
        let parser = ResponseParser::new(Capa);

        let data = "+OK\r\nUSER\r\nRESP-CODES\r\nSASL GSSAPI SKEY\r\n.\r\n";

        let (output, response) = parser.parse(data).unwrap();

        assert!(output.is_empty());

        match response.body() {
            ResponseType::Capability(capas) => {
                println!("{:?}", capas);
                assert!(capas.len() == 3)
            }
            _ => {
                unreachable!()
            }
        }
    }
}
