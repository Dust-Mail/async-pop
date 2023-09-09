use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use bytes::Bytes;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Message {
    inner: Bytes,
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = self.lossy();

        write!(f, "{}", message)
    }
}

impl From<&[u8]> for Message {
    fn from(value: &[u8]) -> Self {
        Self {
            inner: Bytes::copy_from_slice(value),
        }
    }
}

impl AsRef<[u8]> for Message {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<Bytes> for Message {
    fn from(value: Bytes) -> Self {
        Self { inner: value }
    }
}

impl Message {
    pub fn raw(&self) -> &[u8] {
        &self.inner
    }

    pub fn lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    pub fn value(&self) -> Result<&str> {
        Ok(std::str::from_utf8(&self.inner)?)
    }
}
