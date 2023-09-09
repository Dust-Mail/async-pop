use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    result, time,
};

use bytes::Bytes;

use crate::error::{Error, Result};

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone)]
pub struct Number {
    inner: Bytes,
}

impl TryInto<usize> for Number {
    type Error = Error;

    fn try_into(self) -> result::Result<usize, Self::Error> {
        self.value()
    }
}

impl From<&[u8]> for Number {
    fn from(value: &[u8]) -> Self {
        Self {
            inner: Bytes::copy_from_slice(value),
        }
    }
}

impl AsRef<[u8]> for Number {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl From<Bytes> for Number {
    fn from(value: Bytes) -> Self {
        Self { inner: value }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let number = self.lossy_str();

        write!(f, "{}", number)
    }
}

impl Number {
    pub fn raw(&self) -> &[u8] {
        &self.inner
    }

    pub fn lossy_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    pub fn value_str(&self) -> Result<&str> {
        Ok(std::str::from_utf8(&self.inner)?)
    }

    pub fn value(&self) -> Result<usize> {
        let string = self.value_str()?;

        let number: usize = string.parse()?;

        Ok(number)
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone)]
pub struct Duration {
    inner: Number,
    to_secs_multiplier: u64,
}

impl Duration {
    pub fn new<N: Into<Number>>(number: N, to_secs_multiplier: u64) -> Self {
        Self {
            inner: number.into(),
            to_secs_multiplier,
        }
    }

    pub fn value(&self) -> Result<time::Duration> {
        let number = self.inner.value()? as u64;

        let duration = time::Duration::from_secs(number * self.to_secs_multiplier);

        Ok(duration)
    }
}
