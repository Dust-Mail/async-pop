use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    result, time,
};

use bytes::Bytes;

use crate::error::{Error, Result};

use super::DataType;

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone)]
/// Represents a Pop3 number data type.
///
/// Get its real value by calling `value()` from the [DataType] trait
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
        let number = self.as_str_lossy();

        write!(f, "{}", number)
    }
}

impl DataType<usize> for Number {
    fn raw(&self) -> &[u8] {
        &self.inner
    }

    fn as_str_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    fn as_str(&self) -> Result<&str> {
        Ok(std::str::from_utf8(&self.inner)?)
    }

    fn value(&self) -> Result<usize> {
        let string = self.as_str()?;

        let number: usize = string.parse()?;

        Ok(number)
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone)]
/// Represents a Pop3 duration data type.
///
/// Get its real value by calling `value()` from the [DataType] trait
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
}

impl Display for Duration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let number = self.as_str_lossy();

        write!(f, "{}", number)
    }
}

impl TryInto<time::Duration> for Duration {
    type Error = Error;

    fn try_into(self) -> result::Result<time::Duration, Self::Error> {
        self.value()
    }
}

impl DataType<time::Duration> for Duration {
    fn raw(&self) -> &[u8] {
        self.inner.raw()
    }

    fn as_str_lossy(&self) -> Cow<'_, str> {
        self.inner.as_str_lossy()
    }

    fn as_str(&self) -> Result<&str> {
        self.inner.as_str()
    }

    fn value(&self) -> Result<time::Duration> {
        let number = self.inner.value()? as u64;

        let duration = time::Duration::from_secs(number * self.to_secs_multiplier);

        Ok(duration)
    }
}
