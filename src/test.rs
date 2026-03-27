use std::env;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::runtime::net::TcpStream;
use dotenv::dotenv;
use log::info;

use crate::{
    response::{capability::Capability, list::ListResponse, types::DataType, uidl::UidlResponse},
    ClientState,
};

use super::Client;

#[derive(Default)]
struct MockStream {
    read_buf: Vec<u8>,
    read_pos: usize,
    written: Vec<u8>,
}

impl MockStream {
    fn with_response(response: &[u8]) -> Self {
        Self {
            read_buf: response.to_vec(),
            read_pos: 0,
            written: Vec::new(),
        }
    }
}

#[derive(Default)]
struct ChunkedMockStream {
    chunks: Vec<Vec<u8>>,
    chunk_pos: usize,
    written: Vec<u8>,
}

impl ChunkedMockStream {
    fn with_chunks(chunks: &[&[u8]]) -> Self {
        Self {
            chunks: chunks.iter().map(|chunk| chunk.to_vec()).collect(),
            chunk_pos: 0,
            written: Vec::new(),
        }
    }
}

#[cfg(feature = "runtime-tokio")]
impl tokio::io::AsyncRead for MockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let remaining = &self.read_buf[self.read_pos..];
        let len = remaining.len().min(buf.remaining());
        buf.put_slice(&remaining[..len]);
        self.read_pos += len;
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "runtime-tokio")]
impl tokio::io::AsyncRead for ChunkedMockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.chunk_pos >= self.chunks.len() {
            return Poll::Ready(Ok(()));
        }

        let chunk = self.chunks[self.chunk_pos].clone();
        self.chunk_pos += 1;
        buf.put_slice(&chunk);
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "runtime-tokio")]
impl tokio::io::AsyncWrite for MockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "runtime-tokio")]
impl tokio::io::AsyncWrite for ChunkedMockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "runtime-async-std")]
impl async_std::io::Read for MockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let remaining = &self.read_buf[self.read_pos..];
        let len = remaining.len().min(buf.len());
        buf[..len].copy_from_slice(&remaining[..len]);
        self.read_pos += len;
        Poll::Ready(Ok(len))
    }
}

#[cfg(feature = "runtime-async-std")]
impl async_std::io::Read for ChunkedMockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.chunk_pos >= self.chunks.len() {
            return Poll::Ready(Ok(0));
        }

        let chunk = self.chunks[self.chunk_pos].clone();
        self.chunk_pos += 1;

        let len = chunk.len().min(buf.len());
        buf[..len].copy_from_slice(&chunk[..len]);
        Poll::Ready(Ok(len))
    }
}

#[cfg(feature = "runtime-async-std")]
impl async_std::io::Write for MockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "runtime-async-std")]
impl async_std::io::Write for ChunkedMockStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.written.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug)]
struct ClientInfo {
    server: String,
    port: u16,
    username: String,
    password: String,
}

fn create_client_info() -> ClientInfo {
    dotenv().ok();

    ClientInfo {
        server: env::var("SERVER").unwrap().to_owned(),
        port: env::var("PORT").unwrap().parse().unwrap(),
        username: env::var("USERNAME").unwrap().to_owned(),
        password: env::var("PASSWORD").unwrap().to_owned(),
    }
}

async fn create_logged_in_client() -> Client<TcpStream> {
    let client_info = create_client_info();
    let server = client_info.server.as_ref();
    let port = client_info.port;

    let username = client_info.username;
    let password = client_info.password;

    let mut client = super::connect_plain((server, port)).await.unwrap();

    client.login(username, password).await.unwrap();

    client
}

// async fn create_logged_in_client_tls() -> Client<impl crate::tls::TlsStream<TcpStream>> {
//     let client_info = create_client_info();
//     let server = client_info.server.as_ref();
//     let port = client_info.port;

//     let username = client_info.username;
//     let password = client_info.password;

//     let tls = async_native_tls::TlsConnector::new();

//     let mut client = super::connect((server, port), server, &tls).await.unwrap();

//     client.login(username, password).await.unwrap();

//     client
// }

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_connect() {
    let client_info = create_client_info();

    let server = client_info.server.as_ref();
    let port = client_info.port;

    let mut client = super::connect_plain((server, port)).await.unwrap();

    let greeting = client.greeting().unwrap();

    info!("{}", greeting);

    // assert_eq!(greeting, "POP3 GreenMail Server v1.6.12 ready");

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_login() {
    let mut client = create_logged_in_client().await;

    assert_eq!(client.get_state(), &ClientState::Transaction);

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
#[cfg(feature = "sasl")]
async fn e2e_auth() {
    let client_info = create_client_info();

    let server = client_info.server.as_ref();
    let port = client_info.port;

    let mut client = super::connect_plain((server, port)).await.unwrap();

    let plain_auth =
        crate::sasl::PlainAuthenticator::new(client_info.username, client_info.password);

    client.auth(plain_auth).await.unwrap();

    assert_eq!(client.get_state(), &ClientState::Transaction);

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_noop() {
    let mut client = create_logged_in_client().await;

    assert_eq!(client.noop().await.unwrap(), ());

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_stat() {
    let mut client = create_logged_in_client().await;

    let stats = client.stat().await.unwrap();

    assert_eq!(stats.size().value().unwrap(), 0);

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_list() {
    let mut client = create_logged_in_client().await;

    // let list = client.list(Some(1)).await.unwrap();

    let response = client.list(None).await.unwrap();

    match response {
        ListResponse::Multiple(list) => {
            assert_eq!(list.items().len(), 0)
        }
        _ => {
            unreachable!()
        }
    };

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_capa() {
    let mut client = create_logged_in_client().await;

    let capas = client.capa().await.unwrap();

    for capa in capas {
        if let Capability::LoginDelay(time) = capa {
            println!("{}", time.value().unwrap().as_secs())
        }
    }

    client.quit().await.unwrap();
}

// #[cfg_attr(feature = "runtime-tokio", tokio::test)]
// #[cfg_attr(feature = "runtime-async-std", async_std::test)]
// async fn e2e_retr() {

//     let mut client = create_logged_in_client().await;

//     let bytes = client.retr(2).await.unwrap();

//     // println!("{}", String::from_utf8(bytes).unwrap());

//     client.quit().await.unwrap();
// }

// #[cfg_attr(feature = "runtime-tokio", tokio::test)]
// #[cfg_attr(feature = "runtime-async-std", async_std::test)]
// async fn e2e_top() {
//     let mut client = create_logged_in_client().await;

//     let bytes = client.top(3, 0).await.unwrap();

//     println!("{}", std::str::from_utf8(&bytes).unwrap());

//     client.quit().await.unwrap();
// }

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn e2e_uidl() {
    let mut client = create_logged_in_client().await;

    // let uidl = client.uidl(Some(1)).await.unwrap();

    // match uidl {
    //     UidlResponse::Single(unique_id) => {
    //         println!("{}", unique_id.id());
    //     }
    //     _ => {}
    // };

    let uidl = client.uidl(None).await.unwrap();

    match uidl {
        UidlResponse::Multiple(list) => {
            assert_eq!(list.items().len(), 0)
        }
        _ => {
            unreachable!()
        }
    };

    client.quit().await.unwrap();
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn dele_marks_message_as_deleted_locally() {
    let stream = crate::stream::PopStream::new(MockStream::with_response(b"+OK marked\r\n"));

    let mut client = Client {
        inner: Some(stream),
        capabilities: Vec::new(),
        marked_as_del: Vec::new(),
        greeting: Some("ready".into()),
        read_greeting: true,
        state: crate::ClientState::Transaction,
    };

    client.dele(42).await.unwrap();

    assert!(client.is_deleted(&42));
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn split_list_without_argument_still_parses_as_multiline() {
    let stream = crate::stream::PopStream::new(ChunkedMockStream::with_chunks(&[
        b"+OK 1 120\r\n",
        b"2 200\r\n.\r\n",
    ]));

    let mut client = Client {
        inner: Some(stream),
        capabilities: Vec::new(),
        marked_as_del: Vec::new(),
        greeting: Some("ready".into()),
        read_greeting: true,
        state: crate::ClientState::Transaction,
    };

    let response = client.list(None).await.unwrap();

    match response {
        ListResponse::Multiple(list) => {
            assert_eq!(list.items().len(), 2);
        }
        other => panic!("expected multiline LIST response, got {:?}", other),
    }
}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn split_uidl_without_argument_still_parses_as_multiline() {
    let stream = crate::stream::PopStream::new(ChunkedMockStream::with_chunks(&[
        b"+OK 1 whqtswO00WBw418f9t5JxYwZ\r\n",
        b"2 QhdPYR:00WBw1Ph7x7\r\n.\r\n",
    ]));

    let mut client = Client {
        inner: Some(stream),
        capabilities: vec![Capability::Uidl],
        marked_as_del: Vec::new(),
        greeting: Some("ready".into()),
        read_greeting: true,
        state: crate::ClientState::Transaction,
    };

    let response = client.uidl(None).await.unwrap();

    match response {
        UidlResponse::Multiple(list) => {
            assert_eq!(list.items().len(), 2);
        }
        other => panic!("expected multiline UIDL response, got {:?}", other),
    }
}
