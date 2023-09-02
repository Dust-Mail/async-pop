use std::str::{self};

pub mod capability;
pub mod list;
mod parser;
pub mod stat;
pub mod uidl;

use bytes::Bytes;

use crate::{
    command::Command,
    error::{err, ErrorKind, Result},
};

use self::{
    capability::Capability, list::ListResponse, parser::ResponseParser, stat::StatResponse,
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
pub enum ResponseType {
    Stat(StatResponse),
    List(ListResponse),
    Retr(Bytes),
    Noop,
    Top(Bytes),
    Uidl(UidlResponse),
    Capability(Vec<Capability>),
    Message(String),
    Err(String),
}

#[derive(Debug)]
pub struct Response {
    status: Status,
    body: ResponseType,
}

impl Into<ResponseType> for Response {
    fn into(self) -> ResponseType {
        self.body
    }
}

impl Response {
    pub fn new(status: Status, body: ResponseType) -> Self {
        Self { status, body }
    }

    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B, command: Command) -> Result<Self> {
        let input = str::from_utf8(bytes.as_ref())?;

        let parser = ResponseParser::new(command);

        let response = match parser.parse(input) {
            Ok((_, response)) => response,
            Err(err) => {
                err!(
                    ErrorKind::ParseResponse,
                    "Failed to parse POP server response: {}",
                    err
                )
            }
        };

        Ok(response)
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn body(&self) -> &ResponseType {
        &self.body
    }
}
