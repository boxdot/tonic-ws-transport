# tonic-ws-transport
![CI][ci-badge]

An experimental library to enable gRPC tunnelling over a web socket. In
particular, it makes it possible to use streaming gRPC in any browser.

This requires a patched `tonic` which support WASM as compilation target. See
`Cargo.toml` for the patches.

## Why

We would like to use gRPC in browsers that do not support bare TCP connections
and HTTP/2. For that, we use WebSockets to emulate TCP and use HTTP/2
implementation from tonic/hyper/h2. This is especially useful for streaming
gRPC. Note that gRPC-web does not support client to server streaming:
~~[grpc-web#815](https://github.com/grpc/grpc-web/issues/815)~~
[grpc-web#1205](https://github.com/grpc/grpc-web/issues/1205).

## Example

Run server example in native environment:

```bash
cargo run --bin server
```

Then compile wasm and open it in browser:

```bash
cd examples/wasm-client
cargo install wasm-pack
wasm-pack build --target web
python3 -m http.server
# open http://0.0.0.0:8000/ in browser
```

## Usage
You can currently use this as a git dependency. You will need to enable the `tokio_unstable` config flag for tonic to compile.
Update your `.cargo/config.toml`:
```toml
[build]
# This is necessary because tonic needs the tower `make` feature which in turn needs tokios io-std feature which doesn't
# compile on wasm unless tokio_unstable is active
rustflags = ["--cfg=tokio_unstable"]
```

## How

`tonic` has a very flexible API which allows to use custom connectors. A
connector establishes a connection when the client tries to connect to an
endpoint. The server listens on a TCP socket as usual, and uses the same
connection type when a TCP connection is established.

In our cases, instead of connecting to an HTTP resource, we connect to a web
socket, which is supported in native environment but also in all web browsers.
The web socket connector returns a web socket connection which tunnels all
traffic via web socket data frames. In Rust words, it implements
[AsyncWrite](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWrite.html) and
[AsyncRead](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html). The
connection is then used in the `tonic` transport instead of the default TCP
connection.

Finally, the `tonic` client is compiled to WASM. The HTTP/2 (`hyper`) and gRPC
client (`tonic`) implementations are running in WASM. Since web sockets are
streaming by nature, we support all flavors of gRPC, in particular,
(bidirectional) streaming.

The name of this repository is actually a misnomer: technically, the transport
is not `tonic` related. It tunnels any bytes through a web socket, and so can be
used for any binary protocol as transport layer.

## License

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT License ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this document by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

[ci-badge]: https://github.com/boxdot/gurk-rs/workflows/CI/badge.svg
