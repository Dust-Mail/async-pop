use base64::engine::{general_purpose::STANDARD, Engine};
use bytes::Bytes;

use crate::error::Result;

pub fn encode<E: AsRef<[u8]>>(encodable: E) -> String {
    STANDARD.encode(encodable)
}

pub fn decode<E: AsRef<[u8]>>(decodable: E) -> Result<Bytes> {
    Ok(STANDARD.decode(decodable)?.into())
}
