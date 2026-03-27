pub mod capability;
pub mod list;
mod parser;
pub mod stat;
pub mod types;
pub mod uidl;

use bytes::Bytes;
use nom::IResult;

use crate::{command::Command, request::Request};

use self::{
    capability::Capability, list::List, stat::Stat, types::message::Text, uidl::UidlResponse,
};

#[derive(Debug)]
pub struct Status {
    success: bool,
}

impl Status {
    pub fn new(success: bool) -> Self {
        Self { success }
    }

    pub fn success(&self) -> bool {
        self.success
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResponseShape {
    Message,
    Greeting,
    Stat,
    ListSingle,
    ListMulti,
    UidlSingle,
    UidlMulti,
    Bytes,
    Capability,
    #[cfg(feature = "sasl")]
    Auth,
}

impl From<&Request> for ResponseShape {
    fn from(request: &Request) -> Self {
        match request.command() {
            Command::Greet => Self::Greeting,
            Command::Stat => Self::Stat,
            Command::List => {
                if request.arg_count() > 0 {
                    Self::ListSingle
                } else {
                    Self::ListMulti
                }
            }
            Command::Uidl => {
                if request.arg_count() > 0 {
                    Self::UidlSingle
                } else {
                    Self::UidlMulti
                }
            }
            Command::Retr | Command::Top => Self::Bytes,
            Command::Capa => Self::Capability,
            #[cfg(feature = "sasl")]
            Command::Auth | Command::Base64(_) => Self::Auth,
            _ => Self::Message,
        }
    }
}

#[derive(Debug)]
pub enum Response {
    Stat(Stat),
    List(List),
    Bytes(Bytes),
    Uidl(UidlResponse),
    Capability(Vec<Capability>),
    Message(Text),
    #[cfg(feature = "sasl")]
    Challenge(Text),
    Err(Text),
}

impl Response {
    pub fn from_bytes<'a>(input: &'a [u8], command: &Command) -> IResult<&'a [u8], Self> {
        parser::parse(input, command)
    }

    pub(crate) fn from_shape<'a>(
        input: &'a [u8],
        shape: &ResponseShape,
    ) -> IResult<&'a [u8], Self> {
        parser::parse_shape(input, shape)
    }
}
