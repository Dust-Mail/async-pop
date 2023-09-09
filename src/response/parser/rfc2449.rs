use bytes::Bytes;
use nom::{
    branch::alt,
    bytes::streaming::tag_no_case,
    character::streaming::{digit1, one_of, space0, space1},
    combinator::{map, opt, value},
    multi::{many1, many_till, separated_list0},
    sequence::{preceded, terminated},
    IResult,
};

use crate::response::{
    capability::{Capability, Expiration},
    types::number::Duration,
    Response,
};

use super::core::{end_of_multiline, eol, message_parser};

fn sasl_mechanism(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        tag_no_case("KERBEROS_V4"),
        tag_no_case("GSSAPI"),
        tag_no_case("SKEY"),
        tag_no_case("CRAM-MD5"),
        tag_no_case("PLAIN"),
        tag_no_case("XOAUTH2"),
        tag_no_case("OAUTHBEARER"),
    ))(input)
}

fn sasl(input: &[u8]) -> IResult<&[u8], Capability> {
    let (input, _) = tag_no_case("SASL")(input)?;
    let (input, _) = space0(input)?;
    let (input, mechanisms) = separated_list0(space1, sasl_mechanism)(input)?;
    let (input, _) = eol(input)?;

    let capa = Capability::Sasl(
        mechanisms
            .into_iter()
            .map(|slice| Bytes::copy_from_slice(slice))
            .collect(),
    );

    Ok((input, capa))
}

fn login_delay(input: &[u8]) -> IResult<&[u8], Capability> {
    let (input, _) = tag_no_case("LOGIN-DELAY")(input)?;
    let (input, _) = space1(input)?;
    let (input, time) = digit1(input)?;
    let (input, _) = eol(input)?;

    let capa = Capability::LoginDelay(Duration::new(time, 1));

    Ok((input, capa))
}

fn expire(input: &[u8]) -> IResult<&[u8], Capability> {
    let (input, _) = tag_no_case("EXPIRE")(input)?;
    let (input, expiration) = opt(preceded(
        space1,
        alt((
            map(digit1, |time: &[u8]| {
                Expiration::Time(Duration::new(time, 24 * 60 * 60))
            }),
            value(Expiration::Never, tag_no_case("NEVER")),
        )),
    ))(input)?;
    let (input, _) = eol(input)?;

    let capa = Capability::Expire(expiration.unwrap_or_default());

    Ok((input, capa))
}

fn implementation(input: &[u8]) -> IResult<&[u8], Capability> {
    let (input, _) = tag_no_case("IMPLEMENTATION")(input)?;
    let (input, message) = message_parser(input)?;

    let message = message.unwrap_or(b"");

    let capa = Capability::Implementation(Bytes::copy_from_slice(message));

    Ok((input, capa))
}

fn unknown_capability(input: &[u8]) -> IResult<&[u8], Capability> {
    let name = many1(one_of("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-"));

    terminated(
        map(name, |other: Vec<char>| {
            Capability::Other(other.into_iter().map(|byte| byte as u8).collect())
        }),
        eol,
    )(input)
}

fn capability(input: &[u8]) -> IResult<&[u8], Capability> {
    let top = terminated(value(Capability::Top, tag_no_case(b"TOP")), eol);
    let user = terminated(value(Capability::User, tag_no_case(b"USER")), eol);
    let resp_codes = terminated(
        value(Capability::RespCodes, tag_no_case(b"RESP-CODES")),
        eol,
    );
    let pipelining = terminated(
        value(Capability::Pipelining, tag_no_case(b"PIPELINING")),
        eol,
    );
    let uidl = terminated(value(Capability::Uidl, tag_no_case(b"UIDL")), eol);
    let stls = terminated(value(Capability::Stls, tag_no_case(b"STLS")), eol);

    let (input, capability) = alt((
        top,
        user,
        resp_codes,
        sasl,
        login_delay,
        pipelining,
        expire,
        uidl,
        implementation,
        stls,
        unknown_capability,
    ))(input)?;

    Ok((input, capability))
}

pub(crate) fn capability_response(input: &[u8]) -> IResult<&[u8], Response> {
    let (input, _message) = terminated(tag_no_case("Capability list follows"), eol)(input)?;

    let (input, (capabilities, _end)) = many_till(capability, end_of_multiline)(input)?;

    Ok((input, Response::Capability(capabilities)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_expire() {
        let data = b"EXPIRE NEVER\r\n";

        let (input, capa) = capability(data).unwrap();

        assert!(input.is_empty());

        match capa {
            Capability::Expire(expiration) => {
                assert!(expiration == Expiration::Never)
            }
            _ => {
                unreachable!()
            }
        }
    }
}
