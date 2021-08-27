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
