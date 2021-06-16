use super::WsConnection;

use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tonic::transport::server::Connected;
use tungstenite::{Error as TungsteniteError, Message};

use std::io;

impl From<WebSocketStream<TcpStream>> for WsConnection {
    fn from(ws_stream: WebSocketStream<TcpStream>) -> Self {
        use bytes::Bytes;
        use futures_util::{future, SinkExt, StreamExt};

        let peer_addr = ws_stream.get_ref().peer_addr().ok();
        let (sink, stream) = ws_stream.split();

        let sink = sink.sink_err_into();

        let bytes_stream = stream.filter_map(|msg| {
            future::ready(match msg {
                Ok(Message::Binary(data)) => Some(Ok(Bytes::from(data))),
                Ok(Message::Text(data)) => Some(Ok(Bytes::from(data))),
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => None,
                Ok(Message::Close(_)) => Some(Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    TungsteniteError::ConnectionClosed,
                ))),
                Err(e) => Some(Err(io::Error::new(io::ErrorKind::Other, e))),
            })
        });
        let reader = Box::new(tokio_util::io::StreamReader::new(bytes_stream));
        Self {
            sink: Box::new(sink),
            reader,
            peer_addr,
        }
    }
}

#[derive(Clone)]
pub struct WsConnectionInfo {}

impl Connected for WsConnection {
    type ConnectInfo = WsConnectionInfo;

    fn connect_info(&self) -> Self::ConnectInfo {
        WsConnectionInfo {}
    }
}

impl hyper::client::connect::Connection for WsConnection {
    fn connected(&self) -> hyper::client::connect::Connected {
        hyper::client::connect::Connected::new()
    }
}
