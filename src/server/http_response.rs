use anyhow::Context;
use futures::{Stream, TryStreamExt};
use http::{header, Response};
use http::{HeaderValue, StatusCode};
use hyper::Body;
use serde::Serialize;

use crate::server::{HttpResult, HttpRoute};
use crate::server::commons::get_hostname_header;

use super::commons::{BR_CONTENT_ENCODING, DEFLATE_CONTENT_ENCODING, GZIP_CONTENT_ENCODING};

pub struct HttpResponse;

lazy_static! {
    static ref BR_HEADER_VALUE: HeaderValue = HeaderValue::from_static("br");
    static ref DEFLATE_HEADER_VALUE: HeaderValue = HeaderValue::from_static("deflate");
    static ref GZIP_HEADER_VALUE: HeaderValue = HeaderValue::from_static("gzip");
}

impl HttpResponse {
    pub fn internal_server_error(error: anyhow::Error) -> HttpResult {
        let body = Body::from(format!("Error in serving request ==> {:?}", error));

        HttpResponse::build_response(StatusCode::INTERNAL_SERVER_ERROR, body)
    }

    pub fn not_found(reason: &str) -> HttpResult {
        let body = Body::from(format!("Not found: {}", reason));

        HttpResponse::build_response(StatusCode::NOT_FOUND, body)
    }

    pub fn forbidden(reason: &str) -> HttpResult {
        let body = Body::from(format!("Forbidden: {}", reason));

        HttpResponse::build_response(StatusCode::FORBIDDEN, body)
    }

    pub fn bad_request(error: anyhow::Error) -> HttpResult {
        let body = Body::from(format!("Bad Request: {:?}", error));

        HttpResponse::build_response(StatusCode::BAD_REQUEST, body)
    }

    pub fn no_content(reason: &str) -> HttpResult {
        let body = Body::from(format!("No Content: {}", reason));

        HttpResponse::build_response(StatusCode::NO_CONTENT, body)
    }

    fn build_response(code: StatusCode, body: Body) -> HttpResult {
        let response = Response::builder()
            .status(code)
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(response)
    }

    pub fn ok(route: &HttpRoute<'_>, body: Body) -> HttpResult {
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    pub fn string(route: &HttpRoute<'_>, body: String) -> HttpResult {
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::HOST, get_hostname_header().clone())
            .body(Body::from(body))
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    pub fn str(route: &HttpRoute<'_>, body: &'static str) -> HttpResult {
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::HOST, get_hostname_header().clone())
            .body(Body::from(body))
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    pub fn json<S>(route: &HttpRoute<'_>, body: &S) -> HttpResult
        where
            S: Serialize,
    {
        let body = serde_json::to_vec(body).with_context(|| "Error in serialising")?;
        let body = Body::from(body);

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    pub fn proto_binary(route: &HttpRoute<'_>, body: Vec<u8>) -> HttpResult
    {
        let body = Body::from(body);

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "proto/bytes")
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    // TODO: serialise response object to accept format
    pub fn binary_or_json<S>(route: &HttpRoute<'_>, body: &S) -> HttpResult
        where
            S: Serialize,
    {
        let body = serde_json::to_vec(body).with_context(|| "Error in serialising")?;
        let body = Body::from(body);

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    pub fn from_vec<S>(route: &HttpRoute<'_>, body: Vec<u8>) -> HttpResult
        where
            S: Serialize,
    {
        // let body = serde_json::to_vec(body).with_context(|| "Error in serialising")?;
        let body = Body::from(body);

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::HOST, get_hostname_header().clone())
            .body(body)
            .with_context(|| "Error in building HttpResponse")?;

        Ok(Self::compress_response(route, response))
    }

    // pub fn response_visitor<S, F>(
    //     route: &HttpRoute<'_>,
    //     visitor: F,
    // ) -> anyhow::Result<Response<Body>>
    // where
    //     S: Serializer,
    //     F: FnOnce(&mut S) -> Result<S::Ok, S::Error>,
    // {
    //     let mut body = Vec::with_capacity(128);
    //     let mut ser =
    //         serde_json::Serializer::with_formatter(&mut body, serde_json::ser::CompactFormatter);
    //
    //     visitor(&mut ser).with_context(|| "Error in serialising")?;
    //
    //     // let body = serde_json::to_vec(body).with_context(|| "Error in serialising")?;
    //     let body = Body::from(body);
    //
    //     let response = Response::builder()
    //         .status(StatusCode::OK)
    //         .header(header::CONTENT_TYPE, "application/json")
    //         .header(header::HOST, get_hostname_header().clone())
    //         .body(body)
    //         .with_context(|| "Error in building HttpResponse")?;
    //
    //     Ok(Self::compress_response(route, response))
    // }
    //
    // fn response_visitor_inner<S, F>(
    //     route: &HttpRoute<'_>,
    //     serializer: &mut S,
    //     visitor: F,
    // ) -> anyhow::Result<Response<Body>>
    //     where
    //         S: Serializer,
    //         F: FnOnce(&mut S) -> Result<S::Ok, S::Error>,
    // {
    //     visitor(serializer).with_context(|| "Error in serialising")?;
    //
    //     serializer.
    //
    //     // let body = serde_json::to_vec(body).with_context(|| "Error in serialising")?;
    //     let body = Body::from(body);
    //
    //     let response = Response::builder()
    //         .status(StatusCode::OK)
    //         .header(header::CONTENT_TYPE, "application/json")
    //         .header(header::HOST, get_hostname_header().clone())
    //         .body(body)
    //         .with_context(|| "Error in building HttpResponse")?;
    //
    //     Ok(Self::compress_response(route, response))
    // }

    pub fn compress_response(
        route: &HttpRoute<'_>,
        mut response: Response<Body>,
    ) -> Response<Body> {
        use std::io::{Error as IOError, ErrorKind as IOErrorKind};

        // compress as needed
        if let Some(accept_encoding) = route.accept_encoding {
            match accept_encoding {
                BR_CONTENT_ENCODING => {
                    response
                        .headers_mut()
                        .insert(header::CONTENT_ENCODING, BR_HEADER_VALUE.clone());
                    response = response.map(|body| {
                        Body::wrap_stream(brotli_encode(
                            body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                        ))
                    });
                }
                DEFLATE_CONTENT_ENCODING => {
                    response
                        .headers_mut()
                        .insert(header::CONTENT_ENCODING, DEFLATE_HEADER_VALUE.clone());
                    response = response.map(|body| {
                        Body::wrap_stream(deflate_encode(
                            body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                        ))
                    });
                }
                GZIP_CONTENT_ENCODING => {
                    response
                        .headers_mut()
                        .insert(header::CONTENT_ENCODING, GZIP_HEADER_VALUE.clone());
                    response = response.map(|body| {
                        Body::wrap_stream(gzip_encode(
                            body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                        ))
                    });
                }
                _ => {
                    // do nothing
                }
            }
        }

        response
    }
}

fn gzip_encode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::GzipEncoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}

fn brotli_encode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::BrotliEncoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}

fn deflate_encode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::DeflateEncoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}
