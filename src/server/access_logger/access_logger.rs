use anyhow::Context;
use async_trait::async_trait;
use crossbeam_epoch as epoch;
use crossbeam_skiplist::SkipList;
use http::{header, HeaderValue, Response};
use hyper::Body;
use log::{info, Level, log_enabled};
use metered::{HitCount, measure};
use prometheus::{Encoder, Opts, Registry};
use serde::{Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq};

use crate::server::{ApiError, HttpResponse, HttpResult, HttpRoute, Service};
use crate::server::access_logger::metrics::CounterIncrementer;

use super::metrics::ErrorCounter;
use super::response_time::ResponseTime;

lazy_static! {
    pub static ref ACCESS_LOGGER: AccessLogger = AccessLogger::new();
}

pub struct AccessLogger {
    registry: ApiMetricsRegistry,
}

struct ApiMetricsRegistry {
    metrics: SkipList<String, ApiMetrics>,
}

struct ApiMetrics {
    path: String,
    method: String,
    code: u16,
    hits: HitCount,
    errors: ErrorCounter,
    response_time: ResponseTime,
}

impl AccessLogger {
    pub fn new() -> AccessLogger {
        AccessLogger {
            registry: ApiMetricsRegistry {
                metrics: SkipList::new(epoch::default_collector().clone()),
            },
        }
    }

    pub fn log_access(&self, route: &HttpRoute<'_>, response: &HttpResult) {
        let elapsed_time = route.req_instant.elapsed();

        if log_enabled!(target: "access_log", Level::Info) {
            if let Ok(response) = response {
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
        }

        if log_enabled!(target: "metrics_log", Level::Debug) {
            if let Ok(response) = response {
                let path = route.metric_path.unwrap_or_else(|| route.path);
                let code = response.status().as_u16();
                let metric_label = format!("{}/{}/{}", path, route.method, code);

                let guard = &epoch::pin();
                let api_metrics_entry = self.registry.metrics.get_or_insert_with(
                    metric_label,
                    || ApiMetrics {
                        path: path.to_string(),
                        method: route.method.to_string(),
                        code,
                        hits: Default::default(),
                        errors: Default::default(),
                        response_time: Default::default(),
                    },
                    guard,
                );

                let api_metrics: &ApiMetrics = api_metrics_entry.value();

                api_metrics
                    .response_time
                    .increment_time_by_duration(elapsed_time);
                let hits = &api_metrics.hits;
                measure!(hits, {});

                if !response.status().is_success() {
                    api_metrics.errors.increment_by(1);
                }

                api_metrics_entry.release(guard);
            }
        }
    }

    // labels: code, method, path ==> hits, errors, response time
    pub async fn get_api_metrics_for_prometheus(&self, route: &HttpRoute<'_>) -> HttpResult {
        let registry = Registry::new();

        // register 3 counters vector... hits, errors, quantiles... label being ["path", "method", "code"]
        let labels = vec!["path", "method", "code"];
        let hits_counter_opts = Opts::new("hits", "hits counter");
        let hits_counter = prometheus::CounterVec::new(hits_counter_opts, &labels)
            .with_context(|| format!("Error in building hits counter"))?;
        registry
            .register(Box::new(hits_counter.clone()))
            .with_context(|| format!("Error in registering hits counter"))?;

        let errors_counter_opts = Opts::new("errors", "errors counter");
        let errors_counter = prometheus::CounterVec::new(errors_counter_opts, &labels)
            .with_context(|| format!("Error in building errors counter"))?;
        registry
            .register(Box::new(errors_counter.clone()))
            .with_context(|| format!("Error in registering errors counter"))?;

        let quantile_counter_opts = Opts::new("quantiles", "quantiles counter");
        let labels = vec!["path", "method", "code", "quantile"];
        let quantiles_counter = prometheus::CounterVec::new(quantile_counter_opts, &labels)
            .with_context(|| format!("Error in building quantiles counter"))?;
        registry
            .register(Box::new(quantiles_counter.clone()))
            .with_context(|| format!("Error in registering quantiles counter"))?;

        // iterate over registry and serialize
        let guard = &epoch::pin();
        for entry in self.registry.metrics.iter(guard) {
            let api_metrics: &ApiMetrics = entry.value();

            let code = format!("{}", api_metrics.code);
            hits_counter
                .with_label_values(&[&api_metrics.path, &api_metrics.method, &code])
                .inc_by(api_metrics.hits.0.get() as f64);

            errors_counter
                .with_label_values(&[&api_metrics.path, &api_metrics.method, &code])
                .inc_by(api_metrics.errors.0.get() as f64);

            let percentile_map = api_metrics.response_time.get_percentile_map()?;
            for (metric, value) in percentile_map {
                quantiles_counter
                    .with_label_values(&[&api_metrics.path, &api_metrics.method, &code, &metric])
                    .inc_by(value as f64);
            }
        }

        let metric_families = registry.gather();
        let mut buffer = vec![];
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .with_context(|| "Error in encoding prometheus")?;

        HttpResponse::ok(route, Body::from(buffer))
    }

    pub async fn get_api_metrics_as_json(&self, route: &HttpRoute<'_>) -> HttpResult {
        HttpResponse::json(route, &self.registry)
    }
}

impl Serialize for ApiMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("path", &self.path)?;
        map.serialize_entry("method", &self.method)?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("hit_count", &self.hits.0.get())?;
        map.serialize_entry("error_count", &self.errors.0.get())?;
        map.serialize_entry(
            "percentile_metrics",
            &self.response_time.get_percentile_map().unwrap_or_default(),
        )?;
        map.end()
    }
}

impl Serialize for ApiMetricsRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
        where
            S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.metrics.len()))?;

        // iterate over registry and serialize
        let guard = &epoch::pin();

        for entry in self.metrics.iter(guard) {
            let api_metrics: &ApiMetrics = entry.value();

            seq.serialize_element(api_metrics)?;
        }

        seq.end()
    }
}

lazy_static! {
    static ref EMPTY_HEADER_VALUE: HeaderValue = HeaderValue::from_static("");
}

#[async_trait]
impl Service for AccessLogger {
    async fn api_handler<'a>(&'a self, _: Body, route: &HttpRoute<'a>, path: &[&str]) -> Result<Response<Body>, ApiError> {
        match path {
            // sub routes
            ["json"] if matches!(route.method, &http::Method::GET) => self.get_api_metrics_as_json(route).await,

            ["prometheus"] if matches!(route.method, &http::Method::GET) => self.get_api_metrics_for_prometheus(route).await,

            _ => HttpResponse::not_found(route.path),
        }
    }
}