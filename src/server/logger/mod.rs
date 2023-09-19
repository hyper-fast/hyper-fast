#[cfg(feature = "metrics")]
pub use metrics_logger::METRICS_LOGGER;

use crate::server::{HttpResult, HttpRoute};

#[cfg(feature = "access_log")]
mod access_logger;

#[cfg(feature = "metrics")]
mod metrics;

#[cfg(feature = "metrics")]
mod response_time;

#[cfg(feature = "metrics")]
mod metrics_logger;

pub fn log(route: &HttpRoute<'_>, response: &HttpResult) {
    #[cfg(any(feature = "access_log", feature = "metrics"))]
    if let Ok(response) = response {
        let elapsed_time = route.req_instant.elapsed();

        #[cfg(feature = "access_log")]
        access_logger::log(route, response, &elapsed_time);

        #[cfg(feature = "metrics")]
        METRICS_LOGGER.log(route, response, &elapsed_time);
    }
}


