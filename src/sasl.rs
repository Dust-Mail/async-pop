use std::collections::VecDeque;

use async_trait::async_trait;

use crate::{
    command::Command,
    error::{err, ErrorKind, Result},
    request::Request,
    response::{types::message::Text, Response},
    runtime::io::{Read, Write},
    stream::PopStream,
};

/// A simple mechanism to authenticate via PLAIN
pub struct PlainAuthenticator {
    username: String,
    password: String,
}

impl Authenticator for PlainAuthenticator {
    fn mechanism(&self) -> &str {
        "PLAIN"
    }

    fn auth(&self) -> Option<String> {
        Some(format!("\x00{}\x00{}", self.username, self.password))
    }
}

impl PlainAuthenticator {
    pub fn new<U: Into<String>, P: Into<String>>(username: U, password: P) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

/// A simple mechanism to authenticate via OAuth2
pub struct OAuth2Authenticator {
    user: String,
    access_token: String,
}

impl OAuth2Authenticator {
    pub fn new<U: Into<String>, A: Into<String>>(user: U, access_token: A) -> Self {
        Self {
            user: user.into(),
            access_token: access_token.into(),
        }
    }
}

#[async_trait]
impl Authenticator for OAuth2Authenticator {
    fn mechanism(&self) -> &str {
        "XOAUTH2"
    }

    fn auth(&self) -> Option<String> {
        let secret = format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        );

        Some(secret)
    }
}

#[async_trait]
pub trait Authenticator {
    /// The name of the mechanism, e.g: "XOAUTH2" or "KERBEROS_4".
    fn mechanism(&self) -> &str;

    /// If provided, the return string will be added as an argument to the initial "AUTH" command.
    ///
    /// Will automatically be base64 encoded.
    fn auth(&self) -> Option<String> {
        None
    }

    /// Get
    async fn handle<'a, S: Read + Write + Unpin + Send>(
        &self,
        _communicator: Communicator<'a, S>,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct Communicator<'a, S: Read + Write + Unpin + Send> {
    stream: &'a mut PopStream<S>,
    requests: VecDeque<Request>,
}

impl<'a, S: Read + Write + Unpin + Send> Communicator<'a, S> {
    pub fn new(stream: &'a mut PopStream<S>) -> Self {
        Self {
            stream,
            requests: VecDeque::new(),
        }
    }

    pub async fn send<A: Into<String>>(&mut self, secret: A) -> Result<()> {
        let request: Request = Command::Base64(secret.into()).into();

        self.stream.encode(&request).await?;

        self.requests.push_back(request);

        Ok(())
    }

    pub async fn next_challenge(&mut self) -> Result<Text> {
        let command: Command = match self.requests.pop_front() {
            Some(request) => request.into(),
            None => Command::Base64(String::new()),
        };

        let response = self.stream.read_response(command).await?;

        match response {
            Response::Challenge(challenge) => Ok(challenge),
            _ => err!(
                ErrorKind::UnexpectedResponse,
                "Did not get a challenge as a response"
            ),
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.stream.send_bytes("*").await
    }
}
