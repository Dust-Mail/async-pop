pub mod io {
    #[cfg(feature = "runtime-async-std")]
    pub use async_std::io::{prelude::BufReadExt, BufRead, Error, Read, ReadExt, Write, WriteExt};

    #[cfg(feature = "runtime-tokio")]
    pub use tokio::io::{
        AsyncBufRead as BufRead, AsyncBufReadExt as BufReadExt, AsyncRead as Read,
        AsyncReadExt as ReadExt, AsyncWrite as Write, AsyncWriteExt as WriteExt, Error,
    };
}

pub mod net {
    #[cfg(feature = "runtime-async-std")]
    pub use async_std::net::{TcpStream, ToSocketAddrs};

    #[cfg(feature = "runtime-tokio")]
    pub use tokio::net::{TcpStream, ToSocketAddrs};
}

#[cfg(feature = "runtime-async-std")]
pub use async_std::future::timeout;

#[cfg(feature = "runtime-async-std")]
pub use std::time::{Duration, Instant};

#[cfg(feature = "runtime-tokio")]
pub use tokio::time::{timeout, Duration, Instant};
