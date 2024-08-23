use tonic_ws_transport::WsConnection;

use futures_util::StreamExt;
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::server::TcpConnectInfo;
use tonic::{transport::Server, Request, Response, Status};

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
        let addr = request.extensions().get::<TcpConnectInfo>().unwrap();

        println!(
            "Got a request: {:?} from ip {}",
            request,
            addr.remote_addr.unwrap()
        );

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3012";

    let listener = TcpListener::bind(addr).await?;
    let listener_stream = TcpListenerStream::new(listener);
    let incoming = listener_stream.filter_map(|connection| async {
        match connection {
            Ok(tcp_stream) => {
                let info = TcpConnectInfo {
                    local_addr: tcp_stream.local_addr().ok(),
                    remote_addr: tcp_stream.peer_addr().ok(),
                };
                let ws_stream = match tokio_tungstenite::accept_async(tcp_stream).await {
                    Ok(ws_stream) => ws_stream,
                    Err(e) => {
                        eprintln!("failed to accept connection: {e}");
                        return None;
                    }
                };
                Some(Ok(WsConnection::from_combined_channel(ws_stream, info)))
            }
            Err(e) => Some(Err(e)),
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
