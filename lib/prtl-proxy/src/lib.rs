use http::{Request, Response};
use prtl_messages::ProxyDescriptor;

mod serve;
pub mod utils;

pub use prtl_messages as messages;
pub use serve::serve;

#[derive(Debug)]
pub enum Error {
    Http(http::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(e) => write!(f, "HTTP error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<http::Error> for Error {
    fn from(err: http::Error) -> Self {
        Error::Http(err)
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait PrtlService: Send + Sync + 'static {
    fn descriptor(&self) -> ProxyDescriptor;

    async fn handle_request(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, BoxError>;
}
