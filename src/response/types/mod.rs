pub mod message;
pub mod number;

use std::{borrow::Cow, fmt::Display};

use crate::error::Result;

pub trait DataType<T>: Display + TryInto<T> {
    /// Get the actual value that the inner byte slice is representing.
    fn value(&self) -> Result<T>;

    /// Get the value as a raw string, before actual parsing.
    fn as_str(&self) -> Result<&str>;
    /// Get the value as a raw string in lossless fashion, before actual parsing.
    fn as_str_lossy(&self) -> Cow<'_, str>;

    /// The raw slice that represents the data.
    fn raw(&self) -> &[u8];
}
