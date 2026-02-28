use bytes::Bytes;

use super::types::{message::Text, number::Duration};

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone, Default)]
pub enum Expiration {
    #[default]
    Never,
    Time(Duration),
}

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Hash, Clone)]
pub enum Capability {
    /// Whether the TOP command is supported.
    Top,
    /// Whether the USER and PASS commands (login) are supported.
    User,
    /// Whether the use of a SASL based login is supported and if so what kinds. See https://www.rfc-editor.org/rfc/rfc1734
    Sasl(Vec<Bytes>),
    /// Whether the server uses extends response codes. See https://www.rfc-editor.org/rfc/rfc2449#section-8
    RespCodes,
    /// Whether there is a delay between each login and how long it is.
    LoginDelay(Duration),
    /// Whether the server supports pipelining. See https://www.rfc-editor.org/rfc/rfc2197
    Pipelining,
    /// The amount of time the server will store messsages for.
    Expire(Expiration),
    /// Whether the UIDL command is supported.
    Uidl,
    /// The type of authentication method the server prefers/uses.
    Implementation(Text),
    Stls,
    Other(Text),
}

pub type Capabilities = Vec<Capability>;
