use std::net::SocketAddr;
use std::time::Instant;

use chrono::Local;
use http::{header, Method, Request, Uri};
use hyper::Body;

use crate::server::commons::{BR_CONTENT_ENCODING, DEFLATE_CONTENT_ENCODING, GZIP_CONTENT_ENCODING};

pub struct HttpRoute<'a> {
    pub req: &'a Request<Body>,
    pub req_time: chrono::DateTime<Local>,
    pub req_instant: Instant,
    pub method: &'a Method,
    pub uri: &'a Uri,
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub content_encoding: Option<Vec<u8>>,
    pub accept_encoding: Option<&'a [u8]>,
    pub metric_path: Option<&'static str>,
    pub remote_addr: SocketAddr,
}

const CONTENT_ENCODINGS: [&'static [u8]; 3] = [BR_CONTENT_ENCODING, GZIP_CONTENT_ENCODING, DEFLATE_CONTENT_ENCODING];

impl<'a> HttpRoute<'a> {
    pub fn new(req: &'a Request<Body>, req_time: chrono::DateTime<Local>, req_instant: Instant, remote_addr: SocketAddr) -> HttpRoute<'a> {
        HttpRoute {
            req,
            req_time,
            req_instant,
            method: req.method(),
            uri: req.uri(),
            path: req.uri().path(),
            query: req.uri().query(),
            content_encoding: req.headers().get(header::CONTENT_ENCODING).map(|value| value.as_bytes().to_ascii_lowercase()),
            accept_encoding: req.headers().get(header::ACCEPT_ENCODING).and_then(|value| {
                let accept_encodings = value.as_bytes().to_ascii_lowercase();

                for encoding in CONTENT_ENCODINGS.iter() {
                    if let Some(_index) = twoway::find_bytes(&accept_encodings, encoding) {
                        return Some(*encoding);
                    }
                }

                None
            }),
            metric_path: None,
            remote_addr,
        }
    }
}
