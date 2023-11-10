use std::{
    error,
    fmt::{self},
    num::ParseIntError,
    result,
    str::Utf8Error,
};

use crate::runtime::io::Error as IoError;

macro_rules! err {
    ($kind:expr, $($arg:tt)*) => {{
		use crate::error::Error;

        let kind = $kind;
        let message = format!($($arg)*);
        return Err(Error::new( kind, message ));
    }};
}

#[derive(Debug)]
pub enum ErrorKind {
    #[cfg(feature = "async-native-tls")]
    Tls(async_native_tls::Error),
    #[cfg(feature = "async-rustls")]
    InvalidDnsName,
    Io(IoError),
    ParseInt(ParseIntError),
    ParseString(Utf8Error),
    ServerError(String),
    #[cfg(feature = "sasl")]
    DecodeBase64(base64::DecodeError),
    NotConnected,
    ShouldNotBeConnected,
    IncorrectStateForCommand,
    MessageIsDeleted,
    FeatureUnsupported,
    ServerFailedToGreet,
    InvalidResponse,
    ResponseTooLarge,
    MissingRequest,
    ParseCommand,
    UnexpectedResponse,
    ConnectionClosed,
}

#[derive(Debug)]
pub struct Error {
    message: String,
    kind: ErrorKind,
}

impl Error {
    pub fn new<S>(error_kind: ErrorKind, message: S) -> Self
    where
        String: From<S>,
    {
        Self {
            message: message.into(),
            kind: error_kind,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind() {
            _ => None,
        }
    }
}

impl Into<String> for Error {
    fn into(self) -> String {
        self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[cfg(feature = "async-native-tls")]
impl From<async_native_tls::Error> for Error {
    fn from(tls_error: async_native_tls::Error) -> Self {
        Self::new(
            ErrorKind::Tls(tls_error),
            "Error creating secure connection",
        )
    }
}

impl From<IoError> for Error {
    fn from(io_error: IoError) -> Self {
        Self::new(ErrorKind::Io(io_error), "Error with connection to server")
    }
}

impl From<ParseIntError> for Error {
    fn from(parse_int_error: ParseIntError) -> Self {
        Self::new(ErrorKind::ParseInt(parse_int_error), "Failed to parse int")
    }
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Self::new(ErrorKind::ParseString(error), "Failed to parse string")
    }
}

#[cfg(feature = "sasl")]
impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        Self::new(ErrorKind::DecodeBase64(error), "Failed to decode base64")
    }
}

pub(crate) use err;

pub type Result<T> = result::Result<T, Error>;
