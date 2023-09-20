use std::sync::Arc;

use async_trait::async_trait;
use http::Response;
use hyper::Body;

use hyper_fast::server::{ApiError, HttpResponse, HttpRoute, Service};
use hyper_fast::server::{ServiceBuilder, ServiceDaemon, start_http_server};
#[cfg(feature = "settings")]
use hyper_fast::server::utils::load_config;
#[cfg(any(feature = "access_log", feature = "metrics"))]
use hyper_fast::server::utils::setup_logging;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    #[cfg(feature = "settings")]
    load_config("examples/config", "dev")?;

    #[cfg(any(feature = "access_log"))]
    setup_logging("examples/config/log4rs.yml")?;

    start_http_server("127.0.0.1:6464", ExampleServiceBuilder {}).await
}

pub struct ExampleService {
    // any service level properties
}

pub struct ExampleServiceDaemon {}

pub struct ExampleServiceBuilder {
    // any service builder level properties
}

#[async_trait]
impl ServiceDaemon<ExampleService> for ExampleServiceDaemon {
    async fn start(&self, _service: Arc<ExampleService>) {
        //no impl for now.
    }
}

#[async_trait]
impl ServiceBuilder<ExampleService, ExampleServiceDaemon> for ExampleServiceBuilder {
    async fn build(self) -> anyhow::Result<(ExampleService, Option<ExampleServiceDaemon>)> {
        let service = ExampleService {};

        Ok((service, None))
    }
}

#[async_trait]
impl Service for ExampleService {
    async fn api_handler<'a>(
        &'a self,
        _: Body,
        route: &HttpRoute<'a>,
        path: &[&str],
    ) -> Result<Response<Body>, ApiError> {
        match path {
            ["test"] if matches!(route.method, &http::Method::GET) => {
                self.get_test(route).await
            }
            _ => HttpResponse::not_found(route.path),
        }
    }
}

impl ExampleService {
    pub async fn get_test(&self, route: &HttpRoute<'_>) -> Result<Response<Body>, ApiError> {
        HttpResponse::string(route, "GET::/api/test - test passed".to_string())
    }
}

