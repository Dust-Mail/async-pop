use std::str::{self};

pub mod capability;
pub mod list;
mod parser;
pub mod stat;
pub mod uidl;

use bytes::Bytes;

use crate::error::{err, ErrorKind, Result};

use self::{capability::Capability, list::List, stat::StatResponse, uidl::UidlResponse};

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
    Message(String),
    Err(String),
}

impl Response {
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Result<Self> {
        let input = str::from_utf8(bytes.as_ref())?;

        let response = match parser::parse(input) {
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
}
