use bytes::BytesMut;
use futures::{ready, Stream, StreamExt};
use log::trace;
use nom::Needed;
use std::{
    pin::Pin,
    str,
    task::{Context, Poll},
};

use crate::{
    error::{err, ErrorKind},
    macros::escape_newlines,
    request::Request,
    response::{Response, ResponseShape},
    runtime::{
        io::{Read, Write, WriteExt},
        Instant,
    },
};

use crate::{constants::END_OF_LINE, error::Result};

pub struct PopStream<S: Read + Write + Unpin> {
    last_activity: Instant,
    buffer: Buffer,
    decode_needs: usize,
    queue: CommandQueue,
    stream: S,
}

impl<S: Read + Write + Unpin> PopStream<S> {
    /// Send a command to the server and read the response into a string.
    pub async fn encode(&mut self, request: &Request) -> Result<()> {
        self.send_bytes(request.to_string()).await?;

        Ok(())
    }

    /// Send some bytes to the server
    pub async fn send_bytes<B: AsRef<[u8]>>(&mut self, buf: B) -> Result<()> {
        trace!(
            "C: {}",
            escape_newlines!(str::from_utf8(buf.as_ref()).unwrap())
        );

        self.last_activity = Instant::now();

        self.stream.write_all(buf.as_ref()).await?;

        self.stream.write_all(&END_OF_LINE).await?;

        self.stream.flush().await?;

        Ok(())
    }
}

impl<S: Read + Write + Unpin> PopStream<S> {
    fn decode(&mut self) -> Result<Option<Response>> {
        if self.buffer.cursor() < self.decode_needs {
            return Ok(None);
        }

        let used = self.buffer.take();

        let current_shape = self.queue.current();

        match current_shape {
            Some(shape) => {
                match Response::from_shape(&used[..self.buffer.cursor()], shape) {
                    Ok((remaining, response)) => {
                        trace!(
                            "S: {}",
                            escape_newlines!(str::from_utf8(used.as_ref()).unwrap())
                        );

                        self.queue.mark_current_as_done();

                        self.buffer.reset_with(remaining);

                        return Ok(Some(response));
                    }
                    Err(nom::Err::Incomplete(Needed::Size(min))) => {
                        self.decode_needs = self.buffer.cursor() + min.get()
                    }
                    Err(nom::Err::Incomplete(_)) => {
                        self.decode_needs = 0;
                    }
                    Err(other) => {
                        err!(
                            ErrorKind::InvalidResponse,
                            "The server gave an invalid response: '{}'",
                            other
                        )
                    }
                };
            }
            None => {
                self.buffer.return_to(used);

                err!(
                    ErrorKind::MissingRequest,
                    "Trying to read a response without having sent a request"
                );
            }
        }

        self.buffer.return_to(used);

        Ok(None)
    }

    pub async fn read_response<R: Into<Request>>(&mut self, request: R) -> Result<Response> {
        let request = request.into();
        self.queue.add(ResponseShape::from(&request));

        if let Some(resp_result) = self.next().await {
            return match resp_result {
                Ok(resp) => match resp {
                    Response::Err(err) => {
                        err!(ErrorKind::ServerError(err.to_string()), "Server error")
                    }
                    _ => Ok(resp),
                },
                Err(err) => Err(err),
            };
        }

        unreachable!()
    }
}

impl<S: Read + Write + Unpin> Stream for PopStream<S> {
    type Item = Result<Response>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(response) = self.decode()? {
            return Poll::Ready(Some(Ok(response)));
        }

        let this = &mut *self;

        loop {
            this.buffer.ensure_capacity(this.decode_needs)?;

            let buf = this.buffer.unused();

            #[cfg(feature = "runtime-async-std")]
            let bytes_read = ready!(Pin::new(&mut this.stream).poll_read(cx, buf))?;

            #[cfg(feature = "runtime-tokio")]
            let bytes_read = {
                let buf = &mut tokio::io::ReadBuf::new(buf);

                let start = buf.filled().len();

                ready!(Pin::new(&mut this.stream).poll_read(cx, buf))?;

                buf.filled().len() - start
            };

            if bytes_read == 0 {
                return Poll::Ready(Some(Err(crate::error::Error::new(
                    ErrorKind::ConnectionClosed,
                    "The connection was closed by the server",
                ))));
            }

            this.buffer.move_cursor(bytes_read);

            if let Some(response) = this.decode()? {
                return Poll::Ready(Some(Ok(response)));
            }
        }
    }
}

impl<S: Read + Write + Unpin> PopStream<S> {
    pub fn new(stream: S) -> PopStream<S> {
        Self {
            last_activity: Instant::now(),
            buffer: Buffer::new(),
            queue: CommandQueue::new(),
            decode_needs: 0,
            stream,
        }
    }

    pub fn last_activity(&self) -> Instant {
        self.last_activity
    }
}

struct CommandQueue {
    list: Vec<ResponseShape>,
}

impl CommandQueue {
    fn new() -> Self {
        Self { list: Vec::new() }
    }

    fn add(&mut self, shape: ResponseShape) {
        self.list.push(shape)
    }

    fn current(&self) -> Option<&ResponseShape> {
        self.list.first()
    }

    fn mark_current_as_done(&mut self) {
        self.list.remove(0);
    }
}

struct Buffer {
    inner: BytesMut,
    cursor: usize,
}

impl Buffer {
    const CHUNK_SIZE: usize = 2048;
    // Real-world RFC822 messages can easily exceed 20 MiB once attachments are included.
    // Keep a hard cap to avoid unbounded growth, but raise it enough for normal archiving.
    const MAX_SIZE: usize = 128 * 1024 * 1024;

    fn new() -> Self {
        Self {
            cursor: 0,
            inner: BytesMut::zeroed(Self::CHUNK_SIZE),
        }
    }

    fn unused(&mut self) -> &mut [u8] {
        &mut self.inner[self.cursor..]
    }

    fn move_cursor(&mut self, offset: usize) {
        self.cursor += offset;
        if self.cursor > self.inner.len() {
            self.cursor = self.inner.len();
        }
    }

    fn take(&mut self) -> BytesMut {
        std::mem::replace(&mut self.inner, BytesMut::zeroed(Self::CHUNK_SIZE))
    }

    fn return_to(&mut self, inner: BytesMut) {
        self.inner = inner
    }

    fn reset_with<B: AsRef<[u8]>>(&mut self, data: B) {
        let data = data.as_ref();

        self.cursor = data.len();
        self.inner = BytesMut::zeroed(Self::CHUNK_SIZE);
        self.inner[..self.cursor].copy_from_slice(data);
    }

    fn ensure_capacity(&mut self, to_ensure: usize) -> Result<()> {
        let free_bytes: usize = self.inner.len() - self.cursor;

        let extra_bytes_needed: usize = to_ensure.saturating_sub(self.inner.len());

        if free_bytes == 0 || extra_bytes_needed > 0 {
            let increase = std::cmp::max(Self::CHUNK_SIZE, extra_bytes_needed);

            self.grow(increase)?;
        }

        Ok(())
    }

    fn grow(&mut self, amount: usize) -> Result<()> {
        let min_size = self.inner.len() + amount;
        let new_size = match min_size % Self::CHUNK_SIZE {
            0 => min_size,
            n => min_size + (Self::CHUNK_SIZE - n),
        };

        if new_size > Self::MAX_SIZE {
            err!(
                ErrorKind::ResponseTooLarge,
                "The servers response is larger than the maximum allowed size"
            );
        } else {
            self.inner.resize(new_size, 0);

            Ok(())
        }
    }

    fn cursor(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "runtime-tokio")]
    use std::io;

    #[cfg(feature = "runtime-tokio")]
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    #[cfg(feature = "runtime-tokio")]
    #[derive(Default)]
    struct EofThenPanicStream {
        reads: usize,
    }

    #[cfg(feature = "runtime-tokio")]
    impl AsyncRead for EofThenPanicStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            self.reads += 1;
            if self.reads > 1 {
                panic!("poll_read called again after EOF");
            }

            Poll::Ready(Ok(()))
        }
    }

    #[cfg(feature = "runtime-tokio")]
    impl AsyncWrite for EofThenPanicStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[cfg(feature = "runtime-tokio")]
    #[test]
    fn poll_next_returns_connection_closed_on_eof() {
        let mut stream = PopStream::new(EofThenPanicStream::default());
        stream.queue.add(crate::response::ResponseShape::ListMulti);

        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        let poll = Pin::new(&mut stream).poll_next(&mut cx);
        match poll {
            Poll::Ready(Some(Err(err))) => {
                assert!(matches!(err.kind(), ErrorKind::ConnectionClosed));
            }
            other => panic!("expected connection-closed error, got {:?}", other),
        }
    }

    #[test]
    fn buffer_accepts_messages_larger_than_twenty_megabytes() {
        let mut buffer = Buffer::new();
        assert!(buffer.ensure_capacity(25 * 1024 * 1024).is_ok());
    }
}
