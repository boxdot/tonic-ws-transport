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
