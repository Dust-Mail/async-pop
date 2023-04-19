# Pop3 client

This is a simple Pop3 client that implements all of the features according to [RFC 1939](https://www.rfc-editor.org/rfc/rfc1939), written in Rust.

It is used in Dust-Mail to connect to Pop servers.

## Usage

You can create a new session using the `connect` function or the `connect_plain` function.

`connect` expects a tls connector from the `async-native-tls` crate. In the future more tls options will be supported.

If you already have a connected socket, you can also create a new session using the `new` function.

## Example

```rust
extern crate async_pop;
extern crate async_native_tls;
extern crate mailparse;

use async_native_tls::TlsConnector;
use mailparse::parse_mail;

#[tokio::main]
async fn main() {
    let tls = TlsConnector::new();

    let mut client = async_pop::connect(("pop.gmail.com", 995), "pop.gmail.com", &tls, None).await.unwrap();

    client.login("example@gmail.com", "password").await.unwrap();

    let bytes = client.retr(1).await.unwrap();

    let message = parse_mail(&bytes).unwrap();

    let subject = message.headers.get_first_value("Subject").unwrap();

    println!("{}", subject);

}
```
