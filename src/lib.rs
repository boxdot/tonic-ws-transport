use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {}

pub use channel::{Channel, Endpoint};

pub mod channel {
    use crate::Error;

    use bytes::Bytes;
    use http::{uri::InvalidUri, Uri};

    pub struct Channel;

    impl Channel {
        pub fn builder(uri: Uri) -> Endpoint {
            Endpoint
        }

        pub fn from_static(s: &'static str) -> Endpoint {
            Endpoint
        }

        pub fn from_shared(s: impl Into<Bytes>) -> Result<Endpoint, InvalidUri> {
            Ok(Endpoint)
        }
    }

    pub struct Endpoint;

    impl Endpoint {
        pub fn from_static(s: &'static str) -> Self {
            Endpoint
        }

        pub fn from_shared(s: impl Into<Bytes>) -> Result<Self, InvalidUri> {
            Ok(Endpoint)
        }

        pub async fn connect(&self) -> Result<Channel, Error> {
            Ok(Channel)
        }
    }
}

pub mod server {}

pub mod connection {
    use bytes::{BufMut, Bytes};
    use futures_util::{future, StreamExt};
    use http::Uri;
    use pin_project::pin_project;
    use thiserror::Error;
    use tokio::{
        io::{self, AsyncRead},
        net::TcpStream,
    };
    use tokio_tungstenite::{tungstenite, WebSocketStream};

    use std::future::Future;
    use std::mem::MaybeUninit;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Debug, Clone)]
    pub struct WsConnector;

    impl WsConnector {
        pub fn new() -> Self {
            Self
        }

        async fn connect(&mut self, dst: Uri) -> Result<WsConnection, ConnectError> {
            let (ws_stream, _) = tokio_tungstenite::connect_async(dst).await?;
            let (tx, rx) = ws_stream.split();

            let bytes_rx = rx.filter_map(|msg| {
                use tungstenite::Message;
                future::ready(match msg {
                    Ok(Message::Binary(data)) => Some(Ok(Bytes::from(data))),
                    Ok(Message::Text(data)) => Some(Ok(Bytes::from(data))),
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => None,
                    Ok(Message::Close(_)) => Some(Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        tungstenite::error::Error::ConnectionClosed,
                    ))),
                    Err(e) => Some(Err(io::Error::new(tokio::io::ErrorKind::Other, e))),
                })
            });
            let reader = Box::new(tokio::io::stream_reader(bytes_rx));

            Ok(WsConnection { reader })
        }
    }

    #[derive(Debug, Error)]
    pub enum ConnectError {
        #[error(transparent)]
        Tungstenite(#[from] tungstenite::error::Error),
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
        reader: Box<dyn AsyncRead + Unpin>,
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
}
