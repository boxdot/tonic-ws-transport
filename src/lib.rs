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
    use http::Uri;
    use pin_project::pin_project;
    use thiserror::Error;
    use tokio::net::TcpStream;
    use tokio_tungstenite::{tungstenite, WebSocketStream};

    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[derive(Debug, Clone)]
    pub struct WsConnector;

    impl WsConnector {
        pub fn new() -> Self {
            Self
        }

        async fn connect(&mut self, dst: Uri) -> Result<WsConnectorResponse, ConnectError> {
            let (ws_stream, _) = tokio_tungstenite::connect_async(dst).await?;
            Ok(WsConnectorResponse { ws_stream })
        }
    }

    #[derive(Debug, Error)]
    pub enum ConnectError {
        #[error(transparent)]
        Tungstenite(#[from] tungstenite::error::Error),
    }

    impl tower_service::Service<Uri> for WsConnector {
        type Response = WsConnectorResponse;
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

    pub struct WsConnectorResponse {
        ws_stream: WebSocketStream<TcpStream>,
    }

    #[must_use = "futures do nothing unless polled"]
    #[pin_project]
    pub struct WsConnecting {
        #[pin]
        fut: BoxConnecting,
    }

    type ConnectResult = Result<WsConnectorResponse, ConnectError>;
    type BoxConnecting = Pin<Box<dyn Future<Output = ConnectResult> + Send>>;

    impl Future for WsConnecting {
        type Output = ConnectResult;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            self.project().fut.poll(cx)
        }
    }
}
