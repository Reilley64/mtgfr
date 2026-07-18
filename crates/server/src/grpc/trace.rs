//! HTTP-level tower layer: extract W3C `traceparent` and wrap every gRPC call in a tracing span.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskCx, Poll};

use http::{Request, Response};
use opentelemetry::propagation::Extractor;
use tower::{Layer, Service};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct HeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Span for an inbound gRPC HTTP/2 request path (e.g. `/mtgfr.v1.Game/SubmitIntent`).
pub fn span_for_http_request(path: &str, headers: &http::HeaderMap) -> tracing::Span {
    let span = tracing::info_span!(
        "grpc",
        rpc.method = %path,
        table_id = tracing::field::Empty,
        intent.kind = tracing::field::Empty,
        accepted = tracing::field::Empty,
    );
    let parent = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    });
    let _ = span.set_parent(parent);
    span
}

#[derive(Clone, Copy, Default)]
pub struct TraceLayer;

impl<S> Layer<S> for TraceLayer {
    type Service = TraceService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceService { inner }
    }
}

#[derive(Clone)]
pub struct TraceService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for TraceService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskCx<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Clone before call so `poll_ready` reservation stays correct (tower clone pattern).
        let mut inner = self.inner.clone();
        let span = span_for_http_request(req.uri().path(), req.headers());
        Box::pin(async move { inner.call(req).instrument(span).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    #[test]
    fn span_for_http_request_accepts_traceparent_header() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        let mut headers = http::HeaderMap::new();
        headers.insert(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
                .parse()
                .unwrap(),
        );
        let span = span_for_http_request("/mtgfr.v1.Auth/GetMe", &headers);
        // Does not panic; parent link is best-effort when a global tracer is absent.
        let _ = span.enter();
    }

    #[test]
    fn header_extractor_reads_ascii_values() {
        let mut headers = http::HeaderMap::new();
        headers.insert("traceparent", "00-abc-def-01".parse().unwrap());
        let ext = HeaderExtractor(&headers);
        assert_eq!(ext.get("traceparent"), Some("00-abc-def-01"));
        assert!(ext.keys().contains(&"traceparent"));
    }
}
