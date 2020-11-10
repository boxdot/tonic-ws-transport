use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

use tonic_ws_transport::WsConnector;
use wasm_bindgen::prelude::*;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}

#[wasm_bindgen]
pub async fn say_hello() {
    const URL: &str = "ws://127.0.0.1:3012";
    let endpoint = tonic::transport::Endpoint::from_static(URL);
    let channel = endpoint
        .connect_with_connector(WsConnector::new())
        .await
        .expect("failed to connect");
    log::info!("Connected to {}", URL);

    let mut client = GreeterClient::new(channel);

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    log::info!("REQUEST={:?}", request);

    let response = client.say_hello(request).await.expect("RPC call failed");
    log::info!("RESPONSE={:?}", response);
}
