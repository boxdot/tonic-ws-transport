use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;

use tonic_ws_transport::Channel;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = Channel::from_static("http://[::1]:50051");
    let channel = endpoint.connect().await?;

    let mut client = GreeterClient::new(channel);

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
