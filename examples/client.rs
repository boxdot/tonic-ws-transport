use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

use tonic_ws_transport::connection::WsConnector;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = tonic::transport::Endpoint::from_static("ws://127.0.0.1:3012");
    let channel = endpoint.connect_with_connector(WsConnector::new()).await?;

    let mut client = GreeterClient::new(channel);

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
