use std::time::Duration;

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug)]
pub enum Capability {
    /// Whether the TOP command is supported.
    Top,
    /// Whether the USER and PASS commands (login) are supported.
    User,
    /// Whether the use of a SASL based login is supported and if so what kinds. See https://www.rfc-editor.org/rfc/rfc1734
    Sasl(Vec<String>),
    /// Whether the server uses extends response codes. See https://www.rfc-editor.org/rfc/rfc2449#section-8
    RespCodes,
    /// Whether there is a delay between each login and how long it is.
    LoginDelay(Duration),
    /// Whether the server supports pipelining. See https://www.rfc-editor.org/rfc/rfc2197
    Pipelining,
    /// The amount of time the server will store messsages for.
    Expire(Option<Duration>),
    /// Whether the UIDL command is supported.
    Uidl,
    /// The type of authentication method the server prefers/uses.
    Implementation(String),
}

pub type Capabilities = Vec<Capability>;
