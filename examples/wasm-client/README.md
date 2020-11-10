# gRPC over WebSocket wasm example

You'll need [wasm-pack](https://github.com/rustwasm/wasm-pack) to build the wasm module from the
source code.

```
wasm-pack build --target web
```

Then start a local HTTP server in this directory, e.g.

```
python3 -m http.server
```

and navigate to its root in the browser.
