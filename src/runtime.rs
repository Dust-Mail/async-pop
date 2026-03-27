pub mod io {
    #[cfg(feature = "runtime-async-std")]
    pub use async_std::io::{Error, Read, Write, WriteExt};

    #[cfg(feature = "runtime-tokio")]
    pub use tokio::io::{AsyncRead as Read, AsyncWrite as Write, AsyncWriteExt as WriteExt, Error};
}

pub mod net {
    #[cfg(feature = "runtime-async-std")]
    pub use async_std::net::{TcpStream, ToSocketAddrs};

    #[cfg(feature = "runtime-tokio")]
    pub use tokio::net::{TcpStream, ToSocketAddrs};
}

#[cfg(feature = "runtime-async-std")]
pub use std::time::Instant;

#[cfg(feature = "runtime-tokio")]
pub use tokio::time::Instant;
