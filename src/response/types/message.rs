use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    result,
};

use bytes::Bytes;

use crate::error::{Error, Result};

use super::DataType;

#[derive(Debug, Clone)]
/// Represents a Pop3 string data type.
///
/// Get its real value by calling `value()` from the [DataType] trait
pub struct Text {
    inner: Bytes,
}

impl Display for Text {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = self.as_str_lossy();

        write!(f, "{}", message)
    }
}

impl TryInto<String> for Text {
    type Error = Error;

    fn try_into(self) -> result::Result<String, Self::Error> {
        self.value()
    }
}

impl From<&[u8]> for Text {
    fn from(value: &[u8]) -> Self {
        Self {
            inner: Bytes::copy_from_slice(value),
        }
    }
}

impl AsRef<[u8]> for Text {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<Bytes> for Text {
    fn from(value: Bytes) -> Self {
        Self { inner: value }
    }
}

impl DataType<String> for Text {
    fn raw(&self) -> &[u8] {
        &self.inner
    }

    fn as_str_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    fn as_str(&self) -> Result<&str> {
        Ok(std::str::from_utf8(&self.inner)?)
    }

    fn value(&self) -> Result<String> {
        self.as_str().map(|slice| slice.to_string())
    }
}
