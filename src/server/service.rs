use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use async_trait::async_trait;
use http::Response;
use hyper::Body;

use crate::server::{ApiError, HttpRoute};

lazy_static! {
    pub static ref IN_ROTATION: AtomicBool = AtomicBool::new(false);
    pub static ref SHUTDOWN: AtomicBool = AtomicBool::new(false);
}

#[async_trait]
pub trait ServiceBuilder<T: Service, D: ServiceDaemon<T>>: Send + Sync {
    async fn build(self) -> anyhow::Result<(T, Option<D>)>;
}

#[async_trait]
pub trait Service: Send + Sync {
    async fn api_handler<'a>(
        &'a self,
        body: Body,
        route: &HttpRoute<'a>,
        path: &[&str],
    ) -> Result<Response<Body>, ApiError>;
}

#[async_trait]
pub trait ServiceDaemon<T>: Send + Sync
    where
        T: Service,
{
    async fn start(&self, service: Arc<T>);
}
