pub mod capability;
pub mod list;
mod parser;
pub mod stat;
pub mod types;
pub mod uidl;

use bytes::Bytes;
use nom::IResult;

use crate::command::Command;

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
}
