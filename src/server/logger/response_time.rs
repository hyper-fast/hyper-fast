use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

use metered::clear::Clear;
use metered::metric::{Advice, Histogram};
use metered::metric::{Enter, Metric, OnResult};
use metered::time_source::{Instant, StdInstant};
use serde::{Serialize, Serializer};

pub struct HdrHistogram {
    histogram: parking_lot::Mutex<hdrhistogram::Histogram<u64>>,
}

impl Histogram for HdrHistogram {
    fn with_bound(max_bound: u64) -> Self {
        let histogram = hdrhistogram::Histogram::<u64>::new_with_bounds(1, max_bound, 2)
            .expect("Could not instantiate HdrHistogram");
        let histogram = parking_lot::Mutex::new(histogram);
        HdrHistogram { histogram }
    }

    fn record(&self, value: u64) {
        // All recordings will be saturating
        self.histogram.lock().saturating_record(value);
    }
}

impl Clear for HdrHistogram {
    fn clear(&self) {
        self.histogram.lock().reset();
    }
}

impl Serialize for HdrHistogram {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let hdr = &self.histogram.lock();
        let ile = |v| hdr.value_at_percentile(v);
        use serde::ser::SerializeMap;

        let mut tup = serializer.serialize_map(Some(10))?;

        tup.serialize_entry("samples", &hdr.len())?;
        tup.serialize_entry("min", &hdr.min())?;
        tup.serialize_entry("max", &hdr.max())?;
        tup.serialize_entry("mean", &hdr.mean())?;
        tup.serialize_entry("stdev", &hdr.stdev())?;
        tup.serialize_entry("90%ile", &ile(90.0))?;
        tup.serialize_entry("95%ile", &ile(95.0))?;
        tup.serialize_entry("99%ile", &ile(99.0))?;
        tup.serialize_entry("99.9%ile", &ile(99.9))?;
        tup.serialize_entry("99.99%ile", &ile(99.99))?;
        tup.end()
    }
}

impl Debug for HdrHistogram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hdr = &self.histogram.lock();
        let ile = |v| hdr.value_at_percentile(v);
        write!(
            f,
            "HdrHistogram {{
            samples: {}, min: {}, max: {}, mean: {}, stdev: {},
            90%ile = {}, 95%ile = {}, 99%ile = {}, 99.9%ile = {}, 99.99%ile = {} }}",
            hdr.len(),
            hdr.min(),
            hdr.max(),
            hdr.mean(),
            hdr.stdev(),
            ile(90.0),
            ile(95.0),
            ile(99.0),
            ile(99.9),
            ile(99.99)
        )
    }
}

#[derive(Clone)]
pub struct ResponseTime<H: Histogram = HdrHistogram, T: Instant = StdInstant>(
    H,
    std::marker::PhantomData<T>,
);

impl<H: Histogram, T: Instant> Default for ResponseTime<H, T> {
    fn default() -> Self {
        // A HdrHistogram measuring latencies from 1ms to 5minutes
        // All recordings will be saturating, that is, a value higher than 5 minutes
        // will be replace by 5 minutes...
        ResponseTime(H::with_bound(5 * 60 * 1000), std::marker::PhantomData)
    }
}

impl<H: Histogram, T: Instant, R> Metric<R> for ResponseTime<H, T> {}

impl<H: Histogram, T: Instant> Enter for ResponseTime<H, T> {
    type E = T;

    fn enter(&self) -> T {
        T::now()
    }
}

impl<H: Histogram, T: Instant, R> OnResult<R> for ResponseTime<H, T> {
    fn on_result(&self, enter: T, _: &R) -> Advice {
        let elapsed = enter.elapsed_time();
        self.0.record(elapsed);
        Advice::Return
    }
}

impl<H: Histogram, T: Instant> Clear for ResponseTime<H, T> {
    fn clear(&self) {
        self.0.clear();
    }
}

impl<H: Histogram + Serialize, T: Instant> Serialize for ResponseTime<H, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        Serialize::serialize(&self.0, serializer)
    }
}

impl ResponseTime {
    #[allow(dead_code)]
    pub fn increment_time_by(&self, instant: std::time::Instant) {
        let elapsed_micros = instant.elapsed().as_micros() as u64;
        self.0.record(elapsed_micros);
    }

    pub fn increment_time_by_duration(&self, duration: &std::time::Duration) {
        let elapsed_micros = duration.as_micros() as u64;
        self.0.record(elapsed_micros);
    }

    pub fn get_percentile_map(&self) -> anyhow::Result<HashMap<String, u64>> {
        let metrics_required = vec![50.0, 75.0, 90.0, 95.0, 98.0, 99.0, 99.9];
        let mut metric_map: HashMap<String, u64> = HashMap::new();
        let histogram = self.0.histogram.lock();
        for metric in metrics_required {
            let value = histogram.value_at_percentile(metric);
            let metric = metric / 100f64;
            metric_map.insert(format!("{:.3}", metric), value);
        }
        Ok(metric_map)
    }
}

impl<H: Histogram + Debug, T: Instant> Debug for ResponseTime<H, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}
