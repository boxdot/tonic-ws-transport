use tonic_ws_transport::connection::WsConnection;

use futures_util::StreamExt;
use tokio::net::TcpListener;
use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3012";

    let mut listener = TcpListener::bind(addr).await?;
    let incoming = listener.incoming().then(|connection| async {
        match connection {
            Ok(tcp_stream) => {
                let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await.unwrap();
                Ok(WsConnection::from_tungstenite(ws_stream))
            }
            Err(e) => Err(e),
        }
    });

    let greeter = MyGreeter::default();

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve_with_incoming(incoming)
        .await?;

    Ok(())
}
