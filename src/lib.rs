use bytes::{BufMut, Bytes};
use futures_util::{future, ready, sink::Sink, StreamExt};
use http::Uri;
use pin_project::pin_project;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(feature = "native")]
use tokio::net::TcpStream;
#[cfg(feature = "native")]
use tokio_tungstenite::WebSocketStream;
#[cfg(feature = "native")]
use tonic::transport::server::Connected;
use tungstenite::{Error as TungsteniteError, Message};

use std::future::Future;
use std::io;
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Tungstenite(#[from] TungsteniteError),
}

#[derive(Debug, Clone, Default)]
pub struct WsConnector;

impl WsConnector {
    pub fn new() -> Self {
        Default::default()
    }

    async fn connect(&mut self, dst: Uri) -> Result<WsConnection, Error> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "native")] {
                let (ws_stream, _) = tokio_tungstenite::connect_async(dst).await?;
                Ok(WsConnection::from(ws_stream))
            } else {
                todo!();
            }
        }
    }
}

impl tower_service::Service<Uri> for WsConnector {
    type Response = WsConnection;
    type Error = Error;
    type Future = WsConnecting;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
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
    sink: Box<dyn Sink<Message, Error = TungsteniteError> + Send + Unpin>,
    #[pin]
    reader: Box<dyn AsyncRead + Send + Unpin>,
    addr: Option<SocketAddr>,
}

#[cfg(feature = "native")]
impl From<WebSocketStream<TcpStream>> for WsConnection {
    fn from(ws_stream: WebSocketStream<TcpStream>) -> Self {
        let addr = ws_stream.get_ref().remote_addr();
        let (sink, stream) = ws_stream.split();

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
        let reader = Box::new(tokio::io::stream_reader(bytes_stream));
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
        ready!(self_
            .sink
            .as_mut()
            .poll_ready(cx)
            .map_err(from_tungstenite_err)?);
        self_
            .sink
            .start_send(Message::Binary(buf.to_vec()))
            .map_err(from_tungstenite_err)?;
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
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().reader.poll_read(cx, buf)
    }

    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [MaybeUninit<u8>]) -> bool {
        self.reader.prepare_uninitialized_buffer(buf)
    }

    fn poll_read_buf<B: BufMut>(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut B,
    ) -> Poll<io::Result<usize>> {
        self.project().reader.poll_read_buf(cx, buf)
    }
}

#[cfg(feature = "native")]
impl Connected for WsConnection {
    fn remote_addr(&self) -> Option<SocketAddr> {
        self.addr
    }
}

fn from_tungstenite_err(e: TungsteniteError) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}
