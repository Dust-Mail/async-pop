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
pub mod request;
pub mod response;
mod runtime;
mod stream;

#[cfg(feature = "tls")]
mod tls;

#[cfg(feature = "sasl")]
mod base64;
#[cfg(feature = "sasl")]
pub mod sasl;

use std::collections::HashSet;

use bytes::Bytes;
use command::Command::*;
use error::{ErrorKind, Result};
use request::Request;
use response::{
    capability::{Capabilities, Capability},
    list::ListResponse,
    stat::Stat,
    types::message::Text,
    uidl::UidlResponse,
    Response,
};
#[cfg(feature = "sasl")]
use sasl::PlainAuthenticator;
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

pub struct Client<S: Write + Read + Unpin + Send> {
    inner: Option<PopStream<S>>,
    capabilities: Capabilities,
    marked_as_del: Vec<usize>,
    greeting: Option<Text>,
    read_greeting: bool,
    state: ClientState,
}

/// Creates a client from a given socket connection.
async fn create_client_from_socket<S: Read + Write + Unpin + Send>(
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

    client.update_capabilities().await;

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
pub async fn new<S: Read + Write + Unpin + Send>(stream: S) -> Result<Client<S>> {
    let socket = PopStream::new(stream);

    create_client_from_socket(socket).await
}

/// Create a new pop3 client with a tls connection.
#[cfg(feature = "tls")]
pub async fn connect<'a, A: ToSocketAddrs, D: AsRef<str>, C: Into<tls::TlsConnector<'a>>>(
    addr: A,
    domain: D,
    tls: C,
) -> Result<Client<impl tls::TlsStream<TcpStream>>> {
    let tcp_stream = TcpStream::connect(addr).await?;

    let tls_connector: tls::TlsConnector<'a> = tls.into();

    let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

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

impl<S: Read + Write + Unpin + Send> Client<S> {
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
        self.send_request(Noop).await?;

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

        if let Some(msg_number) = msg_number.as_ref() {
            self.check_deleted(msg_number)?
        };

        let mut request: Request = Uidl.into();

        if let Some(number) = msg_number {
            request.add_arg(number)
        }

        let response = self.send_request(request).await?;

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
    ///
    /// Returns [None] if there is no connection or the connection is not in the right state.
    pub fn last_activity(&self) -> Option<Instant> {
        Some(self.inner.as_ref()?.last_activity())
    }

    pub async fn top(&mut self, msg_number: usize, lines: usize) -> Result<Bytes> {
        self.check_deleted(&msg_number)?;

        self.check_capability(vec![Capability::Top])?;

        let mut request: Request = Top.into();

        request.add_arg(msg_number);
        request.add_arg(lines);

        let response = self.send_request(request).await?;

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

        self.marked_as_del.binary_search(msg_number).is_ok()
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

        let mut request: Request = Dele.into();

        request.add_arg(msg_number);

        let response = self.send_request(request).await?;

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
    ///
    /// https://www.rfc-editor.org/rfc/rfc1939#page-9
    pub async fn rset(&mut self) -> Result<Text> {
        let response = self.send_request(Rset).await?;

        self.marked_as_del = Vec::new();

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

        let response = self.send_request(request).await?;

        match response {
            Response::Bytes(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected retr response"
            ),
        }
    }

    /// ## LIST
    ///
    /// If an argument was given and the POP3 server issues a positive response with a line containing information for that message.  This line is called a "scan listing" for that message.
    ///
    /// If no argument was given and the POP3 server issues a positive response, then the response given is multi-line. After the initial +OK, for each message in the maildrop, the POP3 server responds with a line containing information for that message. This line is also called a "scan listing" for that message.  If there are no messages in the maildrop, then the POP3 server responds with no scan listings--it issues a positive response followed by a line containing a termination octet and a CRLF pair.
    ///
    /// ### Arguments:
    /// - a message-number (optional), which, if present, may NOT refer to a message marked as deleted
    /// ### Restrictions:
    /// - may only be given in the TRANSACTION state
    /// ### Possible responses:
    /// - +OK scan listing follows
    /// - -ERR no such message
    pub async fn list(&mut self, msg_number: Option<usize>) -> Result<ListResponse> {
        let mut request: Request = List.into();

        if let Some(msg_number) = msg_number {
            self.check_deleted(&msg_number)?;
            request.add_arg(msg_number)
        }

        let response = self.send_request(request).await?;

        match response {
            Response::List(list) => Ok(list.into()),
            Response::Stat(stat) => Ok(stat.into()),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected list response"
            ),
        }
    }

    /// ## STAT
    /// The POP3 server issues a positive response with a line containing information for the maildrop. This line is called a "drop listing" for that maildrop.
    /// ### Arguments: none
    /// ### Restrictions:
    /// - may only be given in the TRANSACTION state
    /// ### Possible responses:
    /// - +OK nn mm
    pub async fn stat(&mut self) -> Result<Stat> {
        let response = self.send_request(Stat).await?;

        match response {
            Response::Stat(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected stat response"
            ),
        }
    }

    /// ## APOP
    /// Normally, each POP3 session starts with a USER/PASS exchange.  This results in a server/user-id specific password being sent in the clear on the network.  For intermittent use of POP3, this may not introduce a sizable risk.  However, many POP3 client implementations connect to the POP3 server on a regular basis -- to check for new mail.  Further the interval of session initiation may be on the order of five minutes.  Hence, the risk of password capture is greatly enhanced.
    ///
    /// An alternate method of authentication is required which provides for both origin authentication and replay protection, but which does not involve sending a password in the clear over the network.  The APOP command provides this functionality.
    ///
    /// A POP3 server which implements the APOP command will include a timestamp in its banner greeting.  The syntax of the timestamp corresponds to the `msg-id' in [RFC822], and MUST be different each time the POP3 server issues a banner greeting.  For example, on a UNIX implementation in which a separate UNIX process is used for each instance of a POP3 server, the syntax of the timestamp might be:
    ///
    /// `<process-ID.clock@hostname>`
    ///
    /// where `process-ID' is the decimal value of the process's PID, clock is the decimal value of the system clock, and hostname is the fully-qualified domain-name corresponding to the host where the POP3 server is running.
    ///
    /// The POP3 client makes note of this timestamp, and then issues the APOP command.  The `name` parameter has identical semantics to the `name` parameter of the USER command. The `digest` parameter is calculated by applying the MD5 algorithm [RFC1321] to a string consisting of the timestamp (including angle-brackets) followed by a shared
    ///
    /// ### Arguments:
    /// a string identifying a mailbox and a MD5 digest string (both required)
    ///
    /// ### Restrictions:
    /// may only be given in the AUTHORIZATION state after the POP3 greeting or after an unsuccessful USER or PASS command
    ///
    /// ### Possible responses:
    /// - +OK maildrop locked and ready
    /// - -ERR permission denied
    pub async fn apop<N: AsRef<str>, D: AsRef<str>>(&mut self, name: N, digest: D) -> Result<Text> {
        self.check_client_state(ClientState::Authentication)?;

        self.has_read_greeting()?;

        let mut request: Request = Apop.into();

        request.add_arg(name.as_ref());
        request.add_arg(digest.as_ref());

        let response = self.send_request(request).await?;

        self.update_capabilities().await;

        self.state = ClientState::Transaction;

        match response {
            Response::Message(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected apop response"
            ),
        }
    }

    pub fn has_auth_mechanism<M: AsRef<[u8]>>(&self, mechanism: M) -> bool {
        for capa in &self.capabilities {
            if let Capability::Sasl(supported_mechanisms) = capa {
                for supported_mechanism in supported_mechanisms {
                    if supported_mechanism.to_ascii_lowercase()
                        == mechanism.as_ref().to_ascii_lowercase()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// ### AUTH
    ///
    /// Requires an [sasl::Authenticator] to work. One could implement this themeselves for any given mechanism, look at the documentation for this trait.
    ///
    /// If a common mechanism is needed, it can probably be found in the [sasl] module.
    ///
    /// The AUTH command indicates an authentication mechanism to the server.  If the server supports the requested authentication mechanism, it performs an authentication protocol exchange to authenticate and identify the user. Optionally, it also negotiates a protection mechanism for subsequent protocol interactions.  If the requested authentication mechanism is not supported, the server should reject the AUTH command by sending a negative response.
    ///
    /// The authentication protocol exchange consists of a series of server challenges and client answers that are specific to the authentication mechanism.  A server challenge, otherwise known as a ready response, is a line consisting of a "+" character followed by a single space and a BASE64 encoded string.  The client answer consists of a line containing a BASE64 encoded string.  If the client wishes to cancel an authentication exchange, it should issue a line with a single "*".  If the server receives such an answer, it must reject the AUTH command by sending a negative response.
    ///
    /// A protection mechanism provides integrity and privacy protection to the protocol session.  If a protection mechanism is negotiated, it is applied to all subsequent data sent over the connection.  The protection mechanism takes effect immediately following the CRLF that concludes the authentication exchange for the client, and the CRLF of the positive response for the server.  Once the protection mechanism is in effect, the stream of command and response octets is processed into buffers of ciphertext.  Each buffer is transferred over the connection as a stream of octets prepended with a four octet field in network byte order that represents the length of the following data. The maximum ciphertext buffer length is defined by the protection mechanism.
    ///
    /// The server is not required to support any particular authentication mechanism, nor are authentication mechanisms required to support any protection mechanisms.  If an AUTH command fails with a negative response, the session remains in the AUTHORIZATION state and client may try another authentication mechanism by issuing another AUTH command, or may attempt to authenticate by using the USER/PASS or APOP commands.  In other words, the client may request authentication types in decreasing order of preference, with the USER/PASS or APOP command as a last resort.
    #[cfg(feature = "sasl")]
    pub async fn auth<A: sasl::Authenticator + Sync>(&mut self, authenticator: A) -> Result<Text> {
        self.check_client_state(ClientState::Authentication)?;

        self.has_read_greeting()?;

        let mut request: Request = Auth.into();

        let mechanism = authenticator.mechanism();

        request.add_arg(mechanism);

        if let Some(arg) = authenticator.auth() {
            request.add_arg(crate::base64::encode(arg))
        }

        let stream = self.inner_mut()?;

        stream.encode(&request).await?;

        let communicator = sasl::Communicator::new(stream);

        authenticator.handle(communicator).await?;

        let message = match stream.read_response(request).await? {
            Response::Message(message) => message,
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected auith response"
            ),
        };

        self.update_capabilities().await;

        self.state = ClientState::Transaction;

        Ok(message)
    }

    /// ## USER & PASS
    ///
    /// To authenticate using the USER and PASS command combination, the client must first issue the USER command. If the POP3 server responds with a positive status indicator ("+OK"), then the client may issue either the PASS command to complete the authentication, or the QUIT command to terminate the POP3 session.  If the POP3 server responds with a negative status indicator ("-ERR") to the USER command, then the client may either issue a new authentication command or may issue the QUIT command.
    ///
    /// The server may return a positive response even though no such mailbox exists. The server may return a negative response if mailbox exists, but does not permit plaintext password authentication.
    ///
    /// When the client issues the PASS command, the POP3 server uses the argument pair from the USER and PASS commands to determine if the client should be given access to the appropriate maildrop.
    ///
    /// Since the PASS command has exactly one argument, a POP3 server may treat spaces in the argument as part of the password, instead of as argument separators.
    ///
    /// ### Arguments:
    /// -  a string identifying a mailbox (required), which is of significance ONLY to the server
    /// -  a server/mailbox-specific password (required)
    ///
    /// ### Restrictions:
    /// may only be given in the AUTHORIZATION state after the POP3 greeting or after an unsuccessful USER or PASS command
    ///
    /// ### Possible responses:
    /// - +OK maildrop locked and ready
    /// - -ERR invalid password
    /// - -ERR unable to lock maildrop
    /// - -ERR never heard of mailbox name
    pub async fn login<U: AsRef<str>, P: AsRef<str>>(
        &mut self,
        user: U,
        password: P,
    ) -> Result<(Text, Text)> {
        self.check_client_state(ClientState::Authentication)?;

        #[cfg(feature = "sasl")]
        if self.has_auth_mechanism("PLAIN") {
            let plain_auth = PlainAuthenticator::new(user.as_ref(), password.as_ref());

            if let Ok(text) = self.auth(plain_auth).await {
                return Ok((text, Bytes::new().into()));
            }
        }

        self.has_read_greeting()?;

        let mut request: Request = User.into();

        request.add_arg(user.as_ref());

        let user_response = self.send_request(request).await?;

        let mut request: Request = Pass.into();

        request.add_arg(password.as_ref());

        let pass_response = self.send_request(request).await?;

        self.update_capabilities().await;

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
        let response = self.send_request(Quit).await?;

        self.state = ClientState::Update;
        self.inner = None;
        self.state = ClientState::None;
        self.read_greeting = false;

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
        let response = self.send_request(Capa).await?;

        match response {
            Response::Capability(resp) => Ok(resp),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not received the expected capa response"
            ),
        }
    }

    async fn update_capabilities(&mut self) {
        if let Ok(capabilities) = self.capa().await {
            self.capabilities = capabilities
        }
    }

    /// Sends a valid Pop3 command and returns the response sent by the server.
    pub async fn send_request<R: Into<Request>>(&mut self, request: R) -> Result<Response> {
        let request = request.into();

        let stream = self.inner_mut()?;

        stream.encode(&request).await?;

        let response = stream.read_response(request).await?;

        Ok(response)
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

        let response = socket.read_response(Greet).await?;

        match response {
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
        self.greeting.as_ref()
    }
}

#[cfg(test)]
mod test;
