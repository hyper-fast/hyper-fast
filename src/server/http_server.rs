use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use http::{Method, Request};
use hyper::Body;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::server::{HttpResult, IN_ROTATION, Service, ServiceBuilder, ServiceDaemon, SHUTDOWN};

use super::health_check::{get_in_rotation_status, oor_handler};
use super::http_response::HttpResponse;
use super::HttpRoute;
#[cfg(any(feature = "access_log", feature = "metrics"))]
use super::logger;
#[cfg(feature = "metrics")]
use super::logger::METRICS_LOGGER;

fn index(route: &HttpRoute<'_>) -> HttpResult {
    let body = Body::from("Hello, World!");
    HttpResponse::ok(route, body)
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    info!("Installing server shutdown signal");

    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
    IN_ROTATION.store(false, std::sync::atomic::Ordering::Relaxed);

    warn!("Received server shutdown signal");
    std::process::exit(0);
}

// TODO: payload limit - json_payload_limit_conf()
async fn route_handler<App>(
    mut req: Request<Body>,
    remote_addr: SocketAddr,
    app: Arc<App>,
) -> HttpResult
    where
        App: 'static + Service,
{
    let req_time = chrono::Local::now();
    let req_instant = Instant::now();

    let req_body = mem::replace(req.body_mut(), Body::empty());
    let route = HttpRoute::new(&req, req_time, req_instant, remote_addr);

    let parts: Vec<_> = route
        .path
        .split("/")
        .filter(|part| !part.is_empty())
        .collect();

    let response = match &parts[..] {
        [] if matches!(route.method, &Method::GET) => index(&route),
        ["oor"] => oor_handler(&route),
        ["health"] if matches!(route.method, &Method::GET) => get_in_rotation_status(&route),

        #[cfg(feature = "metrics")]
        ["metrics", rest @ ..] => METRICS_LOGGER.api_handler(req_body, &route, rest).await,

        ["api", rest @ ..] => app.api_handler(req_body, &route, rest).await,
        _ => HttpResponse::not_found(route.path),
    };

    #[cfg(feature = "response_time")]
        let response = match response {
        Ok(mut response) => {
            let time_taken = format!("{}", humantime::Duration::from(req_instant.elapsed()));
            let time_taken_header = http::HeaderValue::from_str(&time_taken)
                .with_context(|| format!("Error in building header value time_taken"))?;
            response
                .headers_mut()
                .append("X-time-taken", time_taken_header);
            Ok(response)
        }
        Err(err) => err.into(),
    };

    // log & metrics
    #[cfg(any(feature = "access_log", feature = "metrics"))]
    logger::log_api(&route, &response);

    response
}

pub async fn start_http_server<App, AppDaemon, AppBuilder>(
    addr: &str,
    app_builder: AppBuilder,
) -> anyhow::Result<()>
    where
        App: 'static + Service,
        AppDaemon: 'static + ServiceDaemon<App>,
        AppBuilder: 'static + ServiceBuilder<App, AppDaemon>,
{
    info!("Starting server at addr: {}", addr);

    let addr = addr
        .parse::<SocketAddr>()
        .with_context(|| format!("Parsing node addr '{}' as SocketAddr", addr))?;

    let (app, app_daemon) = app_builder
        .build()
        .await
        .with_context(|| "Error in building app")?;
    let app = Arc::new(app);

    if let Some(app_daemon) = app_daemon {
        // TODO: capture join handle and use in shutdown signal
        let cloned_app = app.clone();
        tokio::task::spawn(async move {
            app_daemon.start(cloned_app).await;
        });
    }

    let make_svc = make_service_fn(move |transport: &AddrStream| {
        // TODO: log new connection
        let remote_addr = transport.remote_addr();
        let app = app.clone();

        async move {
            Ok::<_, anyhow::Error>(service_fn(move |req| {
                // Clone again to ensure that client outlives this closure.
                route_handler(req, remote_addr, app.clone())
            }))
        }
    });

    let server = hyper::Server::try_bind(&addr)
        .with_context(|| "Error in binding to address")?
        .http1_keepalive(true)
        .http1_preserve_header_case(true)
        .http1_title_case_headers(true)
        .serve(make_svc);

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    info!("Started server");

    // Run this server for... forever!
    graceful.await.with_context(|| "Error in starting server")
}
