#[cfg(feature = "native")]
pub use native::WsConnectionInfo;

use futures_util::{ready, sink::Sink};
use pin_project::pin_project;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tungstenite::Message;

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "web")]
mod web;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[cfg(any(feature = "native", feature = "web"))]
    #[error(transparent)]
    Tungstenite(#[from] tungstenite::Error),
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
                Ok(WsConnection::from_combined_channel(ws_stream))
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
}

type WsConnectionSink = Box<dyn Sink<Message, Error = Error> + Unpin + Send>;
type WsConnectionReader = Box<dyn AsyncRead + Unpin + Send>;

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
