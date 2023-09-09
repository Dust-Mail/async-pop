mod core;
mod rfc1939;
mod rfc2449;

use nom::{branch::alt, IResult};

use self::{
    rfc1939::{
        error_response, list_response, retr_response, stat_response, status, string_response,
        top_response, uidl_list_response, uidl_response,
    },
    rfc2449::capability_response,
};

use super::Response;

fn is_empty<B: AsRef<[u8]>>(slice: B) -> bool {
    for byte in slice.as_ref() {
        if byte != &0 {
            return false;
        }
    }

    true
}

pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Response> {
    if is_empty(input) {
        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
    }

    let (input, status) = status(input)?;

    if status.success() {
        alt((
            stat_response,
            uidl_response,
            list_response,
            uidl_list_response,
            capability_response,
            top_response,
            retr_response,
            string_response,
        ))(input)
    } else {
        error_response(input)
    }
}

#[cfg(test)]
mod test {
    use crate::response::{types::DataType, uidl::UidlResponse};

    use super::*;

    #[test]
    fn test_list() {
        let data = b"+OK 2 messages (320 bytes)\r\n1 120 more info\r\n2 200 info info\r\n.\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::List(list) => {
                assert!(list.items().len() == 2);
                // assert!(list.message().as_ref() == b"scan listing follows")
            }
            _ => {
                unreachable!()
            }
        }

        let data = b"+OK 2 messages (320 bytes)\r\n1 120\r\n2 200\r\n";

        let result = parse(data);

        assert!(result.is_err());

        let data = b"+OK 1 120\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Stat(stat) => {
                assert!(stat.counter().value().unwrap() == 1 && stat.size().value().unwrap() == 120)
            }
            _ => {
                unreachable!()
            }
        }

        let data = b"+OK 1 120 test\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Stat(stat) => {
                assert!(stat.counter().value().unwrap() == 1 && stat.size().value().unwrap() == 120)
            }
            _ => {
                unreachable!()
            }
        }

        let data = b"+OK 1 \r\n";

        let result = parse(data);

        assert!(result.is_err())
    }

    #[test]
    fn test_stat() {
        let data = b"+OK 20 600\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Stat(stat) => {
                assert!(stat.counter().value().unwrap() == 20);
                assert!(stat.size().value().unwrap() == 600);
            }
            _ => {
                println!("{:?}", response);
                unreachable!()
            }
        }
    }

    #[test]
    fn test_uidl() {
        let data = b"+OK unique-id listing follows\r\n1 whqtswO00WBw418f9t5JxYwZ\r\n2 QhdPYR:00WBw1Ph7x7\r\n.\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Uidl(uidl) => match uidl {
                UidlResponse::Multiple(list) => {
                    println!("{:?}", list);
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
    fn test_string() {
        let data = b"+OK maildrop has 2 messages (320 octets)\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Message(msg) => {
                assert!(msg.as_ref() == b"maildrop has 2 messages (320 octets)")
            }
            _ => {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_capa() {
        let data = b"+OK\r\nUSER\r\nRESP-CODES\r\nEXPIRE 30\r\nSASL GSSAPI SKEY\r\nGOOGLE-TEST-CAPA\r\n.\r\n";

        let (output, response) = parse(data).unwrap();

        assert!(output.is_empty());

        match response {
            Response::Capability(capas) => {
                println!("{:?}", capas);
                assert!(capas.len() == 5)
            }
            _ => {
                unreachable!()
            }
        }
    }
}
