use hyper::Body;
use hyper::Response;

pub use error::ApiError;
pub use http_request::HttpRequest;
pub use http_response::HttpResponse;
pub use http_route::HttpRoute;
pub use http_server::start_http_server;
// pub(crate) use logger::ACCESS_LOGGER;
pub use service::{IN_ROTATION, Service, ServiceBuilder, ServiceDaemon, SHUTDOWN};

pub type ApiResult<R> = Result<R, ApiError>;
pub type HttpResult = Result<Response<Body>, ApiError>;

#[cfg(any(feature = "access_log", feature = "metrics"))]
mod logger;

mod commons;
mod error;
mod health_check;
mod http_request;
mod http_response;
mod http_route;
mod http_server;
mod service;

#[cfg(feature = "settings")]
mod settings;

pub mod utils;
