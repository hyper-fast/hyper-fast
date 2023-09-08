# hyper-fast

Hyper and rust based very fast HTTP Web framework (much faster than actix and other frameworks).

## Features

- Supports brotli, deflate and gzip encoding for request and response
- In-built access logs and metrics for APIs
- Simple APIs to get current metrics - in JSON and Prometheus format
- In-built OOR (Out of rotation API) to take server out of rotation
- In-built Server Health API
- Very simple and fast match pattern based routing.
- Much faster than actix and other web servers out there.
- Support for optional daemon service that gets started on server start and stopped on server shutdown
- In-built server shutdown handling.

## Example

Look at `examples/example_server.rs` for a working example.

1) Define a service class, implement `Service` trait for api routing.

```rust
pub struct ExampleService {
    // any service level properties
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
```

2) Optional service daemon, could be a dummy implementation - if one doesn't need it.

```rust
pub struct ExampleServiceDaemon {}

#[async_trait]
impl ServiceDaemon<ExampleService> for ExampleServiceDaemon {
    async fn start(&self, _service: Arc<ExampleService>) {
        //no impl for now.
    }
}
```

3) Implement `ServiceBuilder` trait

```rust
pub struct ExampleServiceBuilder {
    // any service builder level properties
}

#[async_trait]
impl ServiceBuilder<ExampleService, ExampleServiceDaemon> for ExampleServiceBuilder {
    async fn build(self) -> anyhow::Result<(ExampleService, Option<ExampleServiceDaemon>)> {
        let service = ExampleService {};

        Ok((service, None))
    }
}
```

4) Invoke `start_http_server` in your main method.

```rust
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    load_config("examples/config", "dev")?;
    setup_logging("examples/config/log4rs.yml")?;

    start_http_server("127.0.0.1:6464", ExampleServiceBuilder {}).await
}
```

### APIs

1) `/oor` - switches the in-rotation status of server
2) `/status` - gives in-rotation status of server
3) `/metrics/json` - metrics in JSON format
4) `/metrics/prometheus` - metrics in Prometheus format
5) `/api/<your-api-routes>` - all your api routes are after `/api`



