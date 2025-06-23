use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use once_cell::sync::Lazy;
use prometheus::{Encoder, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder};
use std::time::{Duration, Instant};

static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);
static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    use prometheus::HistogramOpts;
    HistogramVec::new(
        HistogramOpts::new("request_duration_seconds", "LSP request duration"),
        &["method"],
    )
    .unwrap()
});
static ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(Opts::new("errors_total", "Total LSP errors"), &["method"]).unwrap()
});

pub fn init() {
    let _ = REGISTRY.register(Box::new(HISTOGRAM.clone()));
    let _ = REGISTRY.register(Box::new(ERRORS.clone()));
    start_server();
}

pub fn observe(method: &str, dur: Duration) {
    HISTOGRAM
        .with_label_values(&[method])
        .observe(dur.as_secs_f64());
}

pub fn inc_error(method: &str) {
    ERRORS.with_label_values(&[method]).inc();
}

fn start_server() {
    tokio::spawn(async {
        let make_svc = make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle)) });
        let addr = ([127, 0, 0, 1], 9898).into();
        let _ = Server::bind(&addr).serve(make_svc).await;
    });
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    Ok(Response::new(Body::from(buffer)))
}

pub struct Timer {
    start: Instant,
    method: &'static str,
}

impl Timer {
    pub fn new(method: &'static str) -> Self {
        Self {
            start: Instant::now(),
            method,
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        observe(self.method, self.start.elapsed());
    }
}
