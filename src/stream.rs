use bytes::{Bytes, BytesMut};
use log::trace;
use std::str;

use crate::{
    command::Command,
    error::{err, ErrorKind},
    request::Request,
    response::{Response, ResponseType},
    runtime::{
        io::{Read, ReadExt, Write, WriteExt},
        Instant,
    },
};

use crate::{constants::END_OF_LINE, error::Result};

pub struct PopStream<S: Read + Write + Unpin> {
    last_activity: Option<Instant>,
    stream: S,
}

impl<S: Read + Write + Unpin> PopStream<S> {
    pub fn new(stream: S) -> PopStream<S> {
        Self {
            last_activity: None,
            stream,
        }
    }

    /// Send a command to the server and read the response into a string.
    pub async fn send_request<R: Into<Request>>(&mut self, request: R) -> Result<Response> {
        let request = request.into();

        self.send_bytes(request.to_string()).await?;

        self.last_activity = Some(Instant::now());

        self.read_response(request).await
    }

    pub async fn read_response<C: Into<Command>>(&mut self, command: C) -> Result<Response> {
        let resp_bytes = self.read_bytes().await?;

        let response = Response::from_bytes(resp_bytes, command.into())?;

        match response.body() {
            ResponseType::Err(err) => {
                err!(ErrorKind::ServerError, "{}", err)
            }
            _ => {}
        };

        Ok(response)
    }

    async fn read_bytes(&mut self) -> Result<Bytes> {
        let mut buffer = BytesMut::with_capacity(1024);

        let bytes_read = self.stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            err!(
                ErrorKind::ConnectionClosed,
                "The server closed the connection"
            )
        }

        trace!("S: {}", String::from_utf8(buffer.clone().to_vec()).unwrap());

        Ok(buffer.into())
    }

    /// Send some bytes to the server
    async fn send_bytes<B: AsRef<[u8]>>(&mut self, buf: B) -> Result<()> {
        trace!("C: {}", str::from_utf8(buf.as_ref()).unwrap());

        self.stream.write_all(buf.as_ref()).await?;

        self.stream.write_all(&END_OF_LINE).await?;

        self.stream.flush().await?;

        Ok(())
    }

    pub fn last_activity(&self) -> Option<Instant> {
        self.last_activity
    }
}
