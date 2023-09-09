pub mod capability;
pub mod list;
mod parser;
pub mod stat;
pub mod types;
pub mod uidl;

use bytes::Bytes;
use nom::IResult;

use self::{
    capability::Capability, list::List, stat::StatResponse, types::message::Text,
    uidl::UidlResponse,
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
    Stat(StatResponse),
    List(List),
    Bytes(Bytes),
    Uidl(UidlResponse),
    Capability(Vec<Capability>),
    Message(Text),
    Err(Text),
}

impl Response {
    pub fn from_bytes(input: &[u8]) -> IResult<&[u8], Self> {
        parser::parse(input)
    }
}
