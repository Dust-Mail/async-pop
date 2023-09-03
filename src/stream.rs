use byte_pool::BytePool;
use bytes::{BufMut, Bytes, BytesMut};
use futures::Stream;
use lazy_static::lazy_static;
use log::{info, trace};
use std::{
    pin::Pin,
    str,
    task::{Context, Poll},
};

use crate::{
    command::Command,
    error::{err, ErrorKind},
    request::Request,
    response::Response,
    runtime::{
        io::{Read, ReadExt, Write, WriteExt},
        Instant,
    },
};

use crate::{constants::END_OF_LINE, error::Result};

lazy_static! {
    static ref BYTE_POOL: BytePool<Vec<u8>> = BytePool::new();
}

pub struct PopStream<S: Read + Write + Unpin> {
    last_activity: Option<Instant>,
    stream: S,
}

// impl<S: Read + Write + Unpin> PopStream<S> {
//     fn decode() -> Result<Option<Response>> {}
// }

// impl<S: Read + Write + Unpin> Stream for PopStream<S> {
//     type Item = Response;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {}
// }

impl<S: Read + Write + Unpin> PopStream<S> {
    const CHUNK_SIZE: usize = 2048;
    // const MAX_RESPONSE_SIZE: usize = 1024 * 1024 * 10;

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

        self.read_response().await
    }

    pub async fn read_response(&mut self) -> Result<Response> {
        let resp_bytes = self.read_bytes().await?;

        let response = Response::from_bytes(resp_bytes)?;

        match response {
            Response::Err(err) => {
                err!(ErrorKind::ServerError, "{}", err)
            }
            _ => {}
        };

        Ok(response)
    }

    async fn read_bytes(&mut self) -> Result<Bytes> {
        let mut bytes_read;

        let mut buffer = BytesMut::new();

        loop {
            let mut chunk = BYTE_POOL.alloc(Self::CHUNK_SIZE);

            bytes_read = self.stream.read(&mut chunk).await?;

            info!("{}", bytes_read);

            if bytes_read == 0 {
                err!(
                    ErrorKind::ConnectionClosed,
                    "The server closed the connection"
                )
            }

            buffer.put_slice(&chunk[..bytes_read]);

            if bytes_read < Self::CHUNK_SIZE {
                break;
            }
        }

        trace!("S: {}", String::from_utf8(buffer.to_vec()).unwrap());

        Ok(buffer.into())
    }

    /// Send some bytes to the server
    async fn send_bytes<B: AsRef<[u8]>>(&mut self, buf: B) -> Result<()> {
        trace!("C: {}", str::from_utf8(buf.as_ref()).unwrap());

        self.last_activity = Some(Instant::now());

        self.stream.write_all(buf.as_ref()).await?;

        self.stream.write_all(&END_OF_LINE).await?;

        self.stream.flush().await?;

        Ok(())
    }

    pub fn last_activity(&self) -> Option<Instant> {
        self.last_activity
    }
}
