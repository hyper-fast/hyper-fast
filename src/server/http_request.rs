use anyhow::Context;
use bytes::Buf;
use futures::{Stream, TryStreamExt};
use hyper::Body;
use serde::Deserialize;

use super::commons::{BR_CONTENT_ENCODING, DEFLATE_CONTENT_ENCODING, GZIP_CONTENT_ENCODING};
use super::HttpRoute;

pub struct HttpRequest;

impl HttpRequest {
    pub async fn bytes(route: &HttpRoute<'_>, body: Body) -> anyhow::Result<impl Buf> {
        // TODO: validate content length
        // let content_length = route.req.headers().get(header::CONTENT_LENGTH);

        use std::io::{Error as IOError, ErrorKind as IOErrorKind};

        let body = if let Some(content_encoding) = &route.content_encoding {
            match &content_encoding[..] {
                BR_CONTENT_ENCODING => Body::wrap_stream(brotli_decode(
                    body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                )),
                DEFLATE_CONTENT_ENCODING => Body::wrap_stream(deflate_decode(
                    body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                )),
                GZIP_CONTENT_ENCODING => Body::wrap_stream(gzip_decode(
                    body.map_err(|_| IOError::from(IOErrorKind::InvalidData)),
                )),
                _ => {
                    // do nothing
                    body
                }
            }
        } else {
            body
        };

        // Aggregate the body...
        hyper::body::aggregate(body)
            .await
            .with_context(|| "Error in aggregating body")
    }

    pub async fn value<T>(route: &HttpRoute<'_>, body: Body) -> anyhow::Result<T>
        where
            T: for<'de> Deserialize<'de>,
    {
        // TODO: de-serialise based on content type
        // let content_type = route.req.headers().get(header::CONTENT_TYPE);

        let whole_body = Self::bytes(route, body).await?;

        // Decode as JSON...
        serde_json::from_reader(whole_body.reader())
            .with_context(|| "Error in decoding body_as_value")
    }
}

fn gzip_decode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::GzipDecoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}

fn brotli_decode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::BrotliDecoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}

fn deflate_decode(
    input: impl Stream<Item=std::io::Result<bytes::Bytes>>,
) -> impl Stream<Item=std::io::Result<bytes::Bytes>> {
    tokio_util::io::ReaderStream::new(async_compression::tokio::bufread::DeflateDecoder::new(
        tokio_util::io::StreamReader::new(input),
    ))
}
