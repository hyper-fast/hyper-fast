use std::sync::atomic::Ordering;

use http::Method;
use hyper::Body;

use crate::server::HttpResult;

use super::HttpResponse;
use super::HttpRoute;
use super::service::IN_ROTATION;

fn switch_oor_status(route: &HttpRoute<'_>) -> HttpResult {
    let current_value = IN_ROTATION.load(Ordering::Relaxed);
    IN_ROTATION.store(!current_value, Ordering::Relaxed);
    get_in_rotation_status(route)
}

pub fn get_in_rotation_status(route: &HttpRoute<'_>) -> HttpResult {
    let in_rotation = IN_ROTATION.load(Ordering::Relaxed);
    if in_rotation {
        const OK: &str = "OK";

        HttpResponse::ok(route, Body::from(OK))
    } else {
        const NOK: &str = "NOK";

        HttpResponse::internal_server_error(anyhow::anyhow!(NOK))
    }
}

pub fn oor_handler(route: &HttpRoute<'_>) -> HttpResult {
    let method = route.method;

    match method {
        &Method::GET => switch_oor_status(route),
        &Method::POST => switch_oor_status(route),
        _ => HttpResponse::not_found(route.path),
    }
}
