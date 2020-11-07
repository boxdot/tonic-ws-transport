use thiserror::Error;
use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Tungstenite(#[from] TungsteniteError),
}

pub use channel::{Channel, Endpoint};

pub mod channel {
    use crate::Error;

    use bytes::Bytes;
    use http::{uri::InvalidUri, Uri};

    pub struct Channel;

    impl Channel {
        pub fn builder(_uri: Uri) -> Endpoint {
            Endpoint
        }

        pub fn from_static(_s: &'static str) -> Endpoint {
            Endpoint
        }

        pub fn from_shared(_s: impl Into<Bytes>) -> Result<Endpoint, InvalidUri> {
            Ok(Endpoint)
        }
    }

    pub struct Endpoint;

    impl Endpoint {
        pub fn from_static(_s: &'static str) -> Self {
            Endpoint
        }

        pub fn from_shared(_s: impl Into<Bytes>) -> Result<Self, InvalidUri> {
            Ok(Endpoint)
        }

        pub async fn connect(&self) -> Result<Channel, Error> {
            Ok(Channel)
        }
    }
}

pub mod server {
    // use crate::Error;

    // use tokio::prelude::*;

    // pub struct Builder;

    // impl Builder {
    //     pub async fn accept<S>(stream: S) -> Result<Server, Error>
    //     where
    //         S: AsyncRead + AsyncWrite + Unpin,
    //     {
    //         // let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    //         // Server { ws_stream }
    //         todo!();
    //     }
    // }

    // pub struct Server;
}

pub mod connection {
    use bytes::{BufMut, Bytes};
    use futures_util::{future, ready, sink::Sink, StreamExt};
    use http::Uri;
    use pin_project::pin_project;
    use thiserror::Error;
    use tokio::io::{self, AsyncRead, AsyncWrite};
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite::{Error as TungsteniteError, Message};
    use tokio_tungstenite::WebSocketStream;
    use tonic::transport::server::Connected;

    use std::future::Future;
    use std::mem::MaybeUninit;
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Debug, Clone, Default)]
    pub struct WsConnector;

    impl WsConnector {
        pub fn new() -> Self {
            Default::default()
        }

        async fn connect(&mut self, dst: Uri) -> Result<WsConnection, ConnectError> {
            let (ws_stream, _) = tokio_tungstenite::connect_async(dst).await?;
            Ok(WsConnection::from_tungstenite(ws_stream))
        }
    }

    #[derive(Debug, Error)]
    pub enum ConnectError {
        #[error(transparent)]
        Tungstenite(#[from] TungsteniteError),
    }

    impl tower_service::Service<Uri> for WsConnector {
        type Response = WsConnection;
        type Error = ConnectError;
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

    type ConnectResult = Result<WsConnection, ConnectError>;
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

    impl WsConnection {
        pub fn from_tungstenite(ws_stream: WebSocketStream<TcpStream>) -> Self {
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
                    Err(e) => Some(Err(io::Error::new(tokio::io::ErrorKind::Other, e))),
                })
            });
            let reader = Box::new(io::stream_reader(bytes_stream));
            Self {
                sink: Box::new(sink),
                reader,
                addr,
            }
        }
    }

    impl AsyncWrite for WsConnection {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            // TODO: I have no clue if this is actually correct.
            //       This depends on the meaning of the word "written" in the documentation of
            //       `AsyncWrite`.
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

    impl Connected for WsConnection {
        fn remote_addr(&self) -> Option<SocketAddr> {
            self.addr.clone()
        }
    }

    fn from_tungstenite_err(e: TungsteniteError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, e)
    }
}
