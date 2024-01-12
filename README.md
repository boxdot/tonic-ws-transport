# tonic-ws-transport
![CI][ci-badge]

An experimental library to enable gRPC tunnelling over a WebSocket.

This is work in progress and requires a patched `tonic`. See
`Cargo.toml` for the patches.

## Why

We would like to use gRPC in browsers that do not support bare TCP connections
and HTTP/2. For that, we use WebSockets to emulate TCP and use HTTP/2
implementation from tonic/hyper/h2. This is especially useful for streaming
gRPC. Note that gRPC-web does not support client to server streaming:
https://github.com/grpc/grpc-web/issues/815.

## Examplee

Run server example in native environment:

```
cargo run --bin server
```

Then compile wasm and open it in browser:

```
cd examples/wasm-client
wasm-pack --target web
python3 -m http.server
// open http://0.0.0.0:8000/ in browser
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
