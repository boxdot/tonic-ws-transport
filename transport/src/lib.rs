use futures_util::{ready, sink::Sink};
use pin_project::pin_project;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
#[cfg(feature = "native")]
use tokio::net::TcpStream;
#[cfg(feature = "native")]
use tokio_tungstenite::WebSocketStream;
#[cfg(feature = "native")]
use tonic::transport::server::Connected;
use tungstenite::{Error as TungsteniteError, Message};

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "web")]
mod web;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Tungstenite(#[from] TungsteniteError),
    #[error("Js error: {0}")]
    Js(String),
}

impl From<Error> for io::Error {
    fn from(e: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, e)
    }
}

#[cfg(feature = "web")]
impl From<wasm_bindgen::JsValue> for Error {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        let s = js_sys::JSON::stringify(&value)
            .map(String::from)
            .unwrap_or_else(|_| "unknown".to_string());
        Error::Js(s)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WsConnector;

impl WsConnector {
    pub fn new() -> Self {
        Default::default()
    }

    #[cfg(any(feature = "native", feature = "web"))]
    async fn connect(&mut self, dst: http::Uri) -> Result<WsConnection, Error> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "native")] {
                let (ws_stream, _) = tokio_tungstenite::connect_async(dst).await?;
                Ok(WsConnection::from(ws_stream))
            } else if #[cfg(feature = "web")] {
                web::connect(dst).await
            }
        }
    }
}

#[cfg(any(feature = "native", feature = "web"))]
impl tower::Service<http::Uri> for WsConnector {
    type Response = WsConnection;
    type Error = Error;
    type Future = WsConnecting;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: http::Uri) -> Self::Future {
        let mut self_ = self.clone();
        let fut = Box::pin(async move { self_.connect(dst).await });
        WsConnecting { fut }
    }
}

#[must_use = "futures do nothing unless polled"]
#[pin_project]
pub struct WsConnecting {
    #[pin]
    fut: BoxConnecting,
}

type ConnectResult = Result<WsConnection, Error>;

type BoxConnecting = Pin<Box<dyn Future<Output = ConnectResult> + Send>>;

impl Future for WsConnecting {
    type Output = ConnectResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.project().fut.poll(cx)
    }
}

#[pin_project]
pub struct WsConnection {
    #[pin]
    pub(crate) sink: WsConnectionSink,
    #[pin]
    pub(crate) reader: WsConnectionReader,
    pub(crate) addr: Option<SocketAddr>,
}

impl WsConnection {
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.addr
    }
}

type WsConnectionSink = Box<dyn Sink<Message, Error = Error> + Unpin + Send>;
type WsConnectionReader = Box<dyn AsyncRead + Unpin + Send>;

#[cfg(feature = "native")]
impl From<WebSocketStream<TcpStream>> for WsConnection {
    fn from(ws_stream: WebSocketStream<TcpStream>) -> Self {
        use bytes::Bytes;
        use futures_util::{future, SinkExt, StreamExt};

        let addr = ws_stream.get_ref().remote_addr();
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
            addr,
        }
    }
}

impl AsyncWrite for WsConnection {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut self_ = self.project();
        ready!(self_.sink.as_mut().poll_ready(cx)?);
        self_.sink.start_send(Message::Binary(buf.to_vec()))?;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.project()
            .sink
            .poll_flush(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.project()
            .sink
            .poll_close(cx)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
}

// forward AsyncRead impl to the `reader` field
impl AsyncRead for WsConnection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        self.project().reader.poll_read(cx, buf)
    }
}

#[cfg(feature = "native")]
impl Connected for WsConnection {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr()
    }
}

#[cfg(feature = "native")]
impl hyper::client::connect::Connection for WsConnection {
    fn connected(&self) -> hyper::client::connect::Connected {
        hyper::client::connect::Connected::new()
    }
}
