use std::time::Duration;

use http::{header, HeaderValue, Response};
use hyper::Body;
use log::info;

use crate::server::HttpRoute;

lazy_static! {
    static ref EMPTY_HEADER_VALUE: HeaderValue = HeaderValue::from_static("");
}

pub fn log(route: &HttpRoute<'_>, response: &Response<Body>, elapsed_time: &Duration) {
    let time_taken_in_millis = (elapsed_time.as_nanos() as f64) / 1_000_000.0;

    let response_status = response.status().as_u16();
    let query = route.query.unwrap_or_else(|| "");

    // RemoteAddr
    // RequestTime
    // ResponseStatus
    // TimeInMillis
    // ResponseContentLength
    // ResponseContentType
    // ResponseContentEncoding
    // URLPath
    // QueryPath
    // RequestContentLength
    // RequestContentType
    // RequestContentEncoding
    // RequestAcceptEncoding
    info!(target: "access_log", "{} {} {} {:.6} {:?} {:?} {:?} {} {:?} {:?} {:?} {:?} {:?}",
                      route.remote_addr.ip().to_string(),
                      route.req_time.to_rfc3339(),
                      response_status,
                      time_taken_in_millis,
                      response.headers().get(header::CONTENT_LENGTH), // TODO: have mechanism to properly get response size
                      response.headers().get(header::CONTENT_TYPE).unwrap_or_else(|| &EMPTY_HEADER_VALUE),
                      response.headers().get(header::CONTENT_ENCODING).unwrap_or_else(|| &EMPTY_HEADER_VALUE),
                      route.path,
                      query,
                      route.req.headers().get(header::CONTENT_LENGTH).unwrap_or_else(|| &EMPTY_HEADER_VALUE), // TODO: have mechanism to properly get request size
                      route.req.headers().get(header::CONTENT_TYPE).unwrap_or_else(|| &EMPTY_HEADER_VALUE),
                      route.req.headers().get(header::CONTENT_ENCODING).unwrap_or_else(|| &EMPTY_HEADER_VALUE),
                      route.req.headers().get(header::ACCEPT_ENCODING).unwrap_or_else(|| &EMPTY_HEADER_VALUE),
                );
}