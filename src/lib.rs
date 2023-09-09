//!
//! # Pop3 client
//!
//! This is a simple Pop3 client that implements all of the features according to [RFC 1939](https://www.rfc-editor.org/rfc/rfc1939), written in Rust.
//!
//! ## Usage
//!
//! You can create a new session using the `connect` function or the `connect_plain` function.
//!
//! `connect` expects a tls connector from the `async-native-tls` crate. In the future more tls options will be supported.
//!
//! If you already have a connected socket, you can also create a new session using the `new` function.
//!
//! ## Example
//!
//! ```rust,ignore
//! extern crate async_pop;
//! extern crate async_native_tls;
//! extern crate mailparse;
//!
//! use async_native_tls::TlsConnector;
//! use mailparse::parse_mail;
//!
//! #[tokio::main]
//! async fn main() {
//!     let tls = TlsConnector::new();
//!
//!     let mut client = async_pop::connect(("pop.gmail.com", 995), "pop.gmail.com", &tls, None).await.unwrap();
//!
//!     client.login("example@gmail.com", "password").await.unwrap();
//!
//!     let bytes = client.retr(1).await.unwrap();
//!
//!     let message = parse_mail(&bytes).unwrap();
//!
//!     let subject = message.headers.get_first_value("Subject").unwrap();
//!
//!     println!("{}", subject);
//!
//! }
//! ```

mod command;
mod constants;
pub mod error;
mod macros;
mod request;
pub mod response;
mod runtime;
mod stream;

use std::collections::HashSet;

use async_native_tls::{TlsConnector, TlsStream};
use bytes::Bytes;
use command::Command::*;
use error::{Error, ErrorKind, Result};
use request::Request;
use response::{
    capability::{Capabilities, Capability},
    list::ListResponse,
    stat::Stat,
    types::message::Text,
    uidl::UidlResponse,
    Response,
};
use stream::PopStream;

use crate::{
    error::err,
    runtime::{
        io::{Read, Write},
        net::{TcpStream, ToSocketAddrs},
        Instant,
    },
};

#[derive(Eq, PartialEq, Debug)]
pub enum ClientState {
    Authentication,
    Transaction,
    Update,
    None,
}

pub struct Client<S: Write + Read + Unpin> {
    inner: Option<PopStream<S>>,
    capabilities: Capabilities,
    marked_as_del: Vec<usize>,
    greeting: Option<Text>,
    read_greeting: bool,
    state: ClientState,
}

/// Creates a client from a given socket connection.
async fn create_client_from_socket<S: Read + Write + Unpin>(
    socket: PopStream<S>,
) -> Result<Client<S>> {
    let mut client = Client {
        marked_as_del: Vec::new(),
        capabilities: Vec::new(),
        greeting: None,
        read_greeting: false,
        inner: Some(socket),
        state: ClientState::Authentication,
    };

    client.greeting = Some(client.read_greeting().await?);

    client.capabilities = client.capa().await?;

    Ok(client)
}

/// Creates a new pop3 client from an existing stream.
/// # Examples
/// ```rust,ignore
/// extern crate pop3;
/// use std::net::TcpStream;
///
/// fn main() {
///     // Not recommended to use plaintext, just an example.
///     let stream = TcpStream::connect(("outlook.office365.com", 110)).unwrap();
///
///     let mut client = pop3::new(stream).unwrap();
///
///     client.quit().unwrap();
/// }
/// ```
pub async fn new<S: Read + Write + Unpin>(stream: S) -> Result<Client<S>> {
    let socket = PopStream::new(stream);

    create_client_from_socket(socket).await
}

/// Create a new pop3 client with a tls connection.
pub async fn connect<A: ToSocketAddrs, D: AsRef<str>>(
    addr: A,
    domain: D,
    tls_connector: &TlsConnector,
) -> Result<Client<TlsStream<TcpStream>>> {
    let tcp_stream = TcpStream::connect(addr).await?;

    let tls_stream = tls_connector.connect(domain.as_ref(), tcp_stream).await?;

    let socket = PopStream::new(tls_stream);

    create_client_from_socket(socket).await
}

/// Creates a new pop3 client using a plain connection.
///
/// DO NOT USE in a production environment. Your password will be sent over a plain tcp stream which hackers could intercept.
pub async fn connect_plain<A: ToSocketAddrs>(addr: A) -> Result<Client<TcpStream>> {
    let tcp_stream = TcpStream::connect(addr).await?;

    let socket = PopStream::new(tcp_stream);

    create_client_from_socket(socket).await
}

impl<S: Read + Write + Unpin> Client<S> {
    /// Check if the client is in the correct state and return a mutable reference to the tcp connection.
    fn inner_mut(&mut self) -> Result<&mut PopStream<S>> {
        match self.inner.as_mut() {
            Some(socket) => {
                if self.state == ClientState::Transaction
                    || self.state == ClientState::Authentication
                {
                    Ok(socket)
                } else {
                    err!(
                        ErrorKind::ShouldNotBeConnected,
                        "There is a connection, but our state indicates that we should not be connected",
                    )
                }
            }
            None => err!(ErrorKind::NotConnected, "Not connected to any server",),
        }
    }

    pub fn inner(&self) -> &Option<PopStream<S>> {
        &self.inner
    }

    pub fn into_inner(self) -> Option<PopStream<S>> {
        self.inner
    }

    /// Check if the client is in the correct state.
    fn check_client_state(&self, state: ClientState) -> Result<()> {
        if self.state != state {
            err!(
                ErrorKind::IncorrectStateForCommand,
                "The connection is not the right state to use this command",
            )
        } else {
            Ok(())
        }
    }

    /// ## Current client state
    ///
    /// Indicates what state the client is currently in, can be either
    /// Authentication, Transaction, Update or None.
    ///
    /// Some methods are only available in some specified states and will error if run in an incorrect state.
    ///
    /// https://www.rfc-editor.org/rfc/rfc1939#section-3
    pub fn get_state(&self) -> &ClientState {
        &self.state
    }

    /// ## NOOP
    /// The POP3 server does nothing, it merely replies with a positive response.
    /// ### Arguments: none
    /// ### Restrictions:
    /// - May only be given in the TRANSACTION state
    /// ### Possible Responses:
    /// - OK
    /// # Examples:
    /// ```rust,ignore
    /// client.noop()?;
    /// ```
    /// https://www.rfc-editor.org/rfc/rfc1939#page-9
    pub async fn noop(&mut self) -> Result<()> {
        let socket = self.inner_mut()?;

        socket.send_request(Noop).await?;

        Ok(())
    }

    /// ## UIDL
    /// If an argument was given and the POP3 server issues a positive response with a line containing information for that message.
    /// This line is called a "unique-id listing" for that message.
    ///
    /// If no argument was given and the POP3 server issues a positive response, then the response given is multi-line.
    /// After the initial +OK, for each message in the maildrop, the POP3 server responds with a line containing information for that message.          This line is called a "unique-id listing" for that message.
    ///
    /// ### Arguments:
    /// - a message-number (optional), which, if present, may NOT refer to a message marked as deleted.
    ///
    /// ### Restrictions:
    /// - May only be given in the TRANSACTION state.
    ///
    /// ### Possible responses:
    /// - +OK unique-id listing follows
    /// - -ERR no such message
    ///
    /// https://www.rfc-editor.org/rfc/rfc1939#page-12
    pub async fn uidl(&mut self, msg_number: Option<usize>) -> Result<UidlResponse> {
        self.check_capability(vec![Capability::Uidl])?;

        match msg_number.as_ref() {
            Some(msg_number) => self.check_deleted(msg_number)?,
            None => {}
        };

        let socket = self.inner_mut()?;

        let mut request: Request = Uidl.into();

        if let Some(number) = msg_number {
            request.add_arg(number)
        }

        let response = socket.send_request(request).await?;

        match response {
            Response::Uidl(resp) => Ok(resp),
            _ => {
                err!(
                    ErrorKind::UnexpectedResponse,
                    "Did not received the expected uidl response"
                )
            }
        }
    }

    /// When the last communication with the server happened.
    pub fn last_activity(&mut self) -> Result<Option<Instant>> {
        let socket = self.inner_mut()?;

        let last_activity = socket.last_activity();

        Ok(last_activity)
    }

    pub async fn top(&mut self, msg_number: usize, lines: usize) -> Result<Bytes> {
        self.check_deleted(&msg_number)?;

        self.check_capability(vec![Capability::Top])?;

        let socket = self.inner_mut()?;

        let mut request: Request = Top.into();

        request.add_arg(msg_number);
        request.add_arg(lines);

        let response = socket.send_request(request).await?;

        match response {
            Response::Bytes(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected top response"
            ),
        }
    }

    /// Check whether a given message is marked as deleted by the server.
    ///
    /// If this function returns true then the message may still not exist.
    /// # Examples:
    /// ```rust,ignore
    /// let msg_number: u32 = 8;
    /// let is_deleted = client.is_deleted(msg_number);
    /// assert_eq!(is_deleted, false);
    /// ```
    pub fn is_deleted(&mut self, msg_number: &usize) -> bool {
        self.marked_as_del.sort();

        match self.marked_as_del.binary_search(msg_number) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn check_deleted(&mut self, msg_number: &usize) -> Result<()> {
        if self.is_deleted(msg_number) {
            err!(
                ErrorKind::MessageIsDeleted,
                "This message has been marked as deleted and cannot be refenced anymore",
            )
        } else {
            Ok(())
        }
    }

    /// ## DELE
    /// The POP3 server marks the message as deleted.  Any future reference to the message-number associated with the message in a POP3 command generates an error.  The POP3 server does not actually delete the message until the POP3 session enters the UPDATE state.
    /// ### Arguments:
    /// - a message-number (required) which may NOT refer to a message marked as deleted.
    /// ### Restrictions:
    /// - may only be given in the TRANSACTION state
    /// ### Possible Responses:
    /// - OK: message deleted
    /// - ERR: no such message
    /// # Examples
    /// ```rust,ignore
    /// let msg_number: u32 = 8;
    /// let is_deleted = client.is_deleted(msg_number);
    ///
    /// println!("{}", is_deleted);
    /// ```
    pub async fn dele(&mut self, msg_number: usize) -> Result<Text> {
        self.check_deleted(&msg_number)?;

        let socket = self.inner_mut()?;

        let mut request: Request = Dele.into();

        request.add_arg(msg_number);

        let response = socket.send_request(request).await?;

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected dele response"
            ),
        }
    }

    /// ## RSET
    /// If any messages have been marked as deleted by the POP3
    /// server, they are unmarked.
    /// ### Arguments: none
    /// ### Restrictions:
    /// - May only be given in the TRANSACTION state
    /// ### Possible Responses:
    /// - +OK
    /// # Examples:
    /// ```rust,ignore
    /// client.rset().unwrap();
    /// ```
    /// https://www.rfc-editor.org/rfc/rfc1939#page-9
    pub async fn rset(&mut self) -> Result<Text> {
        let socket = self.inner_mut()?;

        let response = socket.send_request(Rset).await?;

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected rset response"
            ),
        }
    }

    /// ## RETR
    /// Retrieves the full RFC822 compliant message from the server and returns it as a byte vector
    /// ### Arguments:
    /// - A message-number (required) which may NOT refer to a message marked as deleted
    /// ### Restrictions:
    /// - May only be given in the TRANSACTION state
    /// ### Possible Responses:
    /// - OK: message follows
    /// - ERR: no such message
    /// # Examples
    /// ```rust,ignore
    /// extern crate mailparse;
    /// use mailparse::parse_mail;
    ///
    /// let response = client.retr(1).unwrap();
    ///
    /// let parsed = parse_mail(&response);
    ///
    /// let subject = parsed.headers.get_first_value("Subject").unwrap();
    ///
    /// println!("{}", subject);
    /// ```
    /// https://www.rfc-editor.org/rfc/rfc1939#page-8
    pub async fn retr(&mut self, msg_number: usize) -> Result<Bytes> {
        self.check_deleted(&msg_number)?;

        let mut request: Request = Retr.into();

        request.add_arg(msg_number);

        let socket = self.inner_mut()?;

        let response = socket.send_request(request).await?;

        match response {
            Response::Bytes(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected retr response"
            ),
        }
    }

    pub async fn list(&mut self, msg_number: Option<usize>) -> Result<ListResponse> {
        if let Some(msg_number) = msg_number.as_ref() {
            self.check_deleted(msg_number)?;
        }

        let mut request: Request = List.into();

        if let Some(msg_number) = msg_number {
            request.add_arg(msg_number)
        }

        let socket = self.inner_mut()?;

        let response = socket.send_request(request).await?;

        match response {
            Response::List(list) => Ok(list.into()),
            Response::Stat(stat) => Ok(stat.into()),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected list response"
            ),
        }
    }

    pub async fn stat(&mut self) -> Result<Stat> {
        let socket = self.inner_mut()?;

        let response = socket.send_request(Stat).await?;

        match response.into() {
            Response::Stat(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected stat response"
            ),
        }
    }

    pub async fn apop<N: AsRef<str>, D: AsRef<str>>(&mut self, name: N, digest: D) -> Result<Text> {
        self.check_client_state(ClientState::Authentication)?;

        self.has_read_greeting()?;

        let socket = self.inner_mut()?;

        let mut request: Request = Apop.into();

        request.add_arg(name.as_ref());
        request.add_arg(digest.as_ref());

        let response = socket.send_request(request).await?;

        self.state = ClientState::Transaction;

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected apop response"
            ),
        }
    }

    pub async fn auth<U: AsRef<str>>(&mut self, token: U) -> Result<Text> {
        self.check_client_state(ClientState::Authentication)?;

        self.check_capability(vec![Capability::Sasl(vec!["XOAUTH2".into()])])?;

        self.has_read_greeting()?;

        let socket = self.inner_mut()?;

        let mut request: Request = Auth.into();

        request.add_arg(token.as_ref());

        let response = socket.send_request(request).await?;

        self.state = ClientState::Transaction;

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected auth response"
            ),
        }
    }

    pub async fn login<U: AsRef<str>, P: AsRef<str>>(
        &mut self,
        user: U,
        password: P,
    ) -> Result<(Text, Text)> {
        self.check_client_state(ClientState::Authentication)?;

        self.check_capability(vec![
            Capability::User,
            // Capability::Sasl(vec![String::from("PLAIN")]),
        ])?;

        self.has_read_greeting()?;

        let socket = self.inner_mut()?;

        let mut request: Request = User.into();

        request.add_arg(user.as_ref());

        let user_response = socket.send_request(request).await?;

        let mut request: Request = Pass.into();

        request.add_arg(password.as_ref());

        let pass_response = socket.send_request(request).await?;

        self.capabilities = self.capa().await?;

        self.state = ClientState::Transaction;

        let user_response_str = match user_response {
            Response::Message(resp) => resp,
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected user response"
            ),
        };

        let pass_response_str = match pass_response {
            Response::Message(resp) => resp,
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected pass response"
            ),
        };

        Ok((user_response_str, pass_response_str))
    }

    /// ## QUIT
    /// Quits the session
    ///
    /// ### Arguments: none
    ///
    /// ### Restrictions: none
    ///
    /// ### Possible Responses:
    /// - +OK
    ///
    /// https://www.rfc-editor.org/rfc/rfc1939#page-5
    pub async fn quit(&mut self) -> Result<Text> {
        let socket = self.inner_mut()?;

        let response = socket.send_request(Quit).await?;

        self.state = ClientState::Update;
        self.inner = None;
        self.state = ClientState::None;

        self.marked_as_del.clear();
        self.capabilities.clear();

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected quit response"
            ),
        }
    }

    /// Check whether the server supports one of the given capabilities.
    pub fn has_capability<C: AsRef<[Capability]>>(&mut self, capabilities: C) -> bool {
        let to_find: HashSet<_> = capabilities.as_ref().iter().collect();
        let server_has: HashSet<_> = self.capabilities.iter().collect();

        let intersect: Vec<_> = server_has.intersection(&to_find).collect();

        intersect.len() == capabilities.as_ref().len()
    }

    /// Make sure the given capabilities are present
    fn check_capability<C: AsRef<[Capability]>>(&mut self, capability: C) -> Result<()> {
        if !self.has_capability(capability) {
            err!(
                ErrorKind::FeatureUnsupported,
                "The remote pop server does not support this command/function",
            )
        } else {
            Ok(())
        }
    }

    /// Returns the current list of capabilities given by the server.
    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    /// Fetches a list of capabilities for the currently connected server and returns it.
    pub async fn capa(&mut self) -> Result<Capabilities> {
        let stream = self.inner_mut()?;

        let response = stream.send_request(Capa).await?;

        match response.into() {
            Response::Capability(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected capa response"
            ),
        }
    }

    fn has_read_greeting(&self) -> Result<()> {
        if !self.read_greeting {
            err!(
                ErrorKind::ServerFailedToGreet,
                "Did not connect to the server correctly, as we did not get a greeting yet",
            )
        } else {
            Ok(())
        }
    }

    async fn read_greeting(&mut self) -> Result<Text> {
        assert!(!self.read_greeting, "Cannot read greeting twice");

        let socket = self.inner_mut()?;

        let response = socket.read_response().await?;

        match response.into() {
            Response::Message(resp) => {
                self.greeting = Some(resp.clone());
                self.read_greeting = true;

                Ok(resp)
            }
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected greeting"
            ),
        }
    }

    /// The greeting that the POP server sent when the connection opened.
    pub fn greeting(&self) -> Option<&Text> {
        match self.greeting.as_ref() {
            Some(greeting) => Some(greeting),
            None => None,
        }
    }
}

#[cfg(test)]
mod test;
