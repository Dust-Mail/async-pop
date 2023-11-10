use crate::{
    error::Result,
    runtime::io::{Read, Write},
};

pub enum TlsConnector<'a> {
    #[cfg(feature = "async-native-tls")]
    NativeTls(&'a async_native_tls::TlsConnector),
    #[cfg(feature = "async-rustls")]
    RustTls(&'a async_rustls::TlsConnector),
}

#[cfg(feature = "async-native-tls")]
impl<'a> From<&'a async_native_tls::TlsConnector> for TlsConnector<'a> {
    fn from(value: &'a async_native_tls::TlsConnector) -> Self {
        Self::NativeTls(value)
    }
}

#[cfg(feature = "async-rustls")]
impl<'a> From<&'a async_rustls::TlsConnector> for TlsConnector<'a> {
    fn from(value: &'a async_rustls::TlsConnector) -> Self {
        Self::RustTls(value)
    }
}

impl TlsConnector<'_> {
    pub async fn connect<S: Read + Write + Unpin + Send, D: AsRef<str>>(
        &self,
        domain: D,
        tcp_stream: S,
    ) -> Result<impl TlsStream<S>> {
        match self {
            #[cfg(feature = "async-native-tls")]
            Self::NativeTls(connector) => {
                Ok(connector.connect(domain.as_ref(), tcp_stream).await?)
            }
            #[cfg(feature = "async-rustls")]
            Self::RustTls(connector) => {
                let server_name: async_rustls::rustls::ServerName = match domain.as_ref().try_into()
                {
                    Ok(domain) => domain,
                    Err(_err) => crate::err!(
                        crate::ErrorKind::InvalidDnsName,
                        "Given domain name '{}' was invalid",
                        domain.as_ref()
                    ),
                };

                Ok(connector.connect(server_name, tcp_stream).await?)
            }
        }
    }
}

pub trait TlsStream<S: Read + Write + Unpin + Send>: Read + Write + Unpin + Send {}

#[cfg(feature = "async-native-tls")]
impl<S: Read + Write + Unpin + Send> TlsStream<S> for async_native_tls::TlsStream<S> {}

#[cfg(feature = "async-rustls")]
impl<S: Read + Write + Unpin + Send> TlsStream<S> for async_rustls::client::TlsStream<S> {}
