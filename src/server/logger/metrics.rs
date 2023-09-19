use std::sync::atomic::Ordering;

use metered::atomic::AtomicInt;
use metered::clear::Clear;
use metered::Enter;
use metered::metric::{Advice, Counter, Metric, OnResult};
use metered::time_source::{Instant, StdInstant};
use serde::{Serialize, Serializer};

pub trait CounterIncrementer: Default + Clear + Serialize {
    fn increment_by(&self, number: u64);
}

impl CounterIncrementer for AtomicInt<u64> {
    fn increment_by(&self, number: u64) {
        self.inner.fetch_add(number, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug)]
pub struct TotalTimeElapsed<A: CounterIncrementer = AtomicInt<u64>, C: Instant = StdInstant>(pub A, pub std::marker::PhantomData<C>);

impl<A: CounterIncrementer, C: Instant, R> Metric<R> for TotalTimeElapsed<A, C> {}

impl<A: CounterIncrementer, C: Instant> Enter for TotalTimeElapsed<A, C> {
    type E = C;
    fn enter(&self) -> Self::E {
        C::now()
    }
}

impl<A: CounterIncrementer, C: Instant> Default for TotalTimeElapsed<A, C> {
    fn default() -> Self {
        TotalTimeElapsed(A::default(), std::marker::PhantomData)
    }
}

impl<A: CounterIncrementer, C: Instant, R> OnResult<R> for TotalTimeElapsed<A, C> {
    fn on_result(&self, enter: C, _: &R) -> Advice {
        let elapsed = enter.elapsed_time();
        self.0.increment_by(elapsed);
        Advice::Return
    }
}

impl<A: CounterIncrementer, C: Instant> Clear for TotalTimeElapsed<A, C> {
    fn clear(&self) {
        self.0.clear()
    }
}

impl<A: CounterIncrementer, C: Instant> Serialize for TotalTimeElapsed<A, C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        Serialize::serialize(&self.0, serializer)
    }
}

#[derive(Clone, Default, Debug, Serialize)]
pub struct ErrorCounter<C: Counter = AtomicInt<u64>>(pub C);

impl<C: Counter> Enter for ErrorCounter<C> {
    type E = ();
    fn enter(&self) {}
}

impl CounterIncrementer for ErrorCounter {
    fn increment_by(&self, number: u64) {
        self.0.inner.fetch_add(number, Ordering::Relaxed);
    }
}

impl<C: Counter, T, E> Metric<Result<T, E>> for ErrorCounter<C> {}

impl<C: Counter, T, E> OnResult<Result<T, E>> for ErrorCounter<C> {
    fn on_result(&self, _: (), r: &Result<T, E>) -> Advice {
        if r.is_err() {
            self.0.incr();
        };
        Advice::Return
    }
}

impl<C: Counter> Clear for ErrorCounter<C> {
    fn clear(&self) {
        self.0.clear()
    }
}

// pub fn register_prometheus_counter(registry: &Registry, name: String, help: String) -> anyhow::Result<PrometheusCounter> {
//     let metric_options = Opts::new(name, help);
//     let metric_counter = PrometheusCounter::with_opts(metric_options)?;
//     registry.register(Box::new(metric_counter.clone()))?;
//     Ok(metric_counter)
// }
//
// pub fn get_percentile_metrics(
//     registry_name: &str,
//     mut label_map: HashMap<String, String>,
//     percentile_map: &HashMap<String, u64>,
// ) -> anyhow::Result<String> {
//     let mut metric_text = String::default();
//     for (metric, value) in percentile_map {
//         // if *value == 0 {
//         //     continue;
//         // }
//         label_map.insert("quantile".to_string(), metric.clone());
//         let registry = Registry::new_custom(Some(registry_name.to_string()), Some(label_map.clone()))?;
//         let metric_counter = register_prometheus_counter(&registry, "quantile".to_string(), "Percentile Metrics".to_string())?;
//         metric_counter.inc_by(*value as f64);
//         metric_text.push_str(&get_prometheus_text(&registry)?)
//     }
//     Ok(metric_text)
// }
//
// pub fn get_prometheus_text(registry: &Registry) -> anyhow::Result<String> {
//     let mut buffer = vec![];
//     let encoder = TextEncoder::new();
//     let metric_families = registry.gather();
//     encoder
//         .encode(&metric_families, &mut buffer)
//         .with_context(|| "Unable to encode prometheus buffer")?;
//
//     String::from_utf8(buffer).with_context(|| "Unable to convert prometheus buffer to text")
// }
//
// pub fn get_average_latency(total_time_taken: &TotalTimeElapsed, hits: &HitCount) -> u64 {
//     if hits.0.get() > 0 {
//         total_time_taken.0.get() / hits.0.get()
//     } else {
//         0
//     }
// }
