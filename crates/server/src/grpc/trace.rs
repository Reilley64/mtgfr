//! HTTP-level tower layer: extract W3C `traceparent` and wrap every gRPC call in a tracing span.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskCx, Poll};

use http::{Request, Response};
use opentelemetry::propagation::Extractor;
use tower::{Layer, Service};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::otel_semconv::{
    RPC_GRPC_STATUS_CODE, RPC_METHOD, RPC_SERVICE, RPC_SYSTEM, parse_grpc_path, rpc_span_name,
};

struct HeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Span for an inbound gRPC HTTP/2 request path (e.g. `/mtgfr.v1.GameService/SubmitIntent`).
pub fn span_for_http_request(path: &str, headers: &http::HeaderMap) -> tracing::Span {
    let (service, method) = parse_grpc_path(path).unwrap_or(("unknown", "unknown"));
    let name = rpc_span_name(service, method);
    let span = tracing::info_span!(
        "grpc",
        otel.name = tracing::field::Empty,
        rpc.system = tracing::field::Empty,
        rpc.service = tracing::field::Empty,
        rpc.method = tracing::field::Empty,
        rpc.grpc.status_code = tracing::field::Empty,
        mtgfr.table.id = tracing::field::Empty,
        mtgfr.intent.kind = tracing::field::Empty,
        mtgfr.intent.accepted = tracing::field::Empty,
        mtgfr.user.id = tracing::field::Empty,
    );
    span.record("otel.name", tracing::field::display(&name));
    span.record(RPC_SYSTEM, "grpc");
    span.record(RPC_SERVICE, service);
    span.record(RPC_METHOD, method);
    let parent = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    });
    let _ = span.set_parent(parent);
    span
}

fn record_grpc_status_from_headers(headers: &http::HeaderMap) {
    let status = headers
        .get("grpc-status")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .unwrap_or("0");
    tracing::Span::current().record(RPC_GRPC_STATUS_CODE, tracing::field::display(status));
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
        Box::pin(async move {
            let result = inner.call(req).instrument(span.clone()).await;
            if let Ok(response) = &result {
                let _enter = span.enter();
                record_grpc_status_from_headers(response.headers());
            }
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt;
    use std::sync::{Arc, Mutex};

    use crate::otel_semconv::{
        MTGFR_INTENT_ACCEPTED, MTGFR_INTENT_KIND, MTGFR_TABLE_ID, MTGFR_USER_ID,
        RPC_GRPC_STATUS_CODE, RPC_METHOD, RPC_SERVICE, RPC_SYSTEM,
    };
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use tracing::field::{Field, Visit};
    use tracing::{Id, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct CapturingLayer {
        observed: Arc<Mutex<Option<ObservedSpan>>>,
    }

    #[derive(Debug, Default)]
    struct ObservedSpan {
        name: String,
        fields: BTreeSet<String>,
        values: BTreeMap<String, String>,
    }

    #[derive(Default)]
    struct FieldVisitor {
        values: BTreeMap<String, String>,
    }

    impl Visit for FieldVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.values
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            self.values
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    impl<S> Layer<S> for CapturingLayer
    where
        S: Subscriber,
    {
        fn on_new_span(
            &self,
            attrs: &tracing::span::Attributes<'_>,
            _id: &Id,
            _ctx: Context<'_, S>,
        ) {
            let mut visitor = FieldVisitor::default();
            attrs.record(&mut visitor);
            let observed = ObservedSpan {
                name: attrs.metadata().name().to_string(),
                fields: attrs
                    .metadata()
                    .fields()
                    .iter()
                    .map(|field| field.name().to_string())
                    .collect(),
                values: visitor.values,
            };
            *self.observed.lock().expect("observed span lock") = Some(observed);
        }

        fn on_record(&self, _span: &Id, values: &tracing::span::Record<'_>, _ctx: Context<'_, S>) {
            let mut visitor = FieldVisitor::default();
            values.record(&mut visitor);
            let mut observed = self.observed.lock().expect("observed span lock");
            if let Some(observed) = observed.as_mut() {
                observed.values.extend(visitor.values);
            }
        }
    }

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
        let span = span_for_http_request("/mtgfr.v1.AuthService/GetMe", &headers);
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

    #[test]
    fn span_for_http_request_uses_rpc_semconv_fields() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        let headers = http::HeaderMap::new();
        let layer = CapturingLayer::default();
        let observed = layer.observed.clone();
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = span_for_http_request("/mtgfr.v1.Auth/GetMe", &headers);
            let _enter = span.enter();
        });

        let observed = observed
            .lock()
            .expect("observed span lock")
            .take()
            .expect("span was created");
        assert_eq!(observed.name, "grpc");
        assert_eq!(
            observed.values.get("otel.name").map(String::as_str),
            Some("mtgfr.v1.Auth/GetMe")
        );
        assert_eq!(
            observed.values.get(RPC_SYSTEM).map(String::as_str),
            Some("grpc")
        );
        assert_eq!(
            observed.values.get(RPC_SERVICE).map(String::as_str),
            Some("mtgfr.v1.Auth")
        );
        assert_eq!(
            observed.values.get(RPC_METHOD).map(String::as_str),
            Some("GetMe")
        );
        assert!(observed.fields.contains(RPC_GRPC_STATUS_CODE));
        assert!(observed.fields.contains(MTGFR_TABLE_ID));
        assert!(observed.fields.contains(MTGFR_INTENT_KIND));
        assert!(observed.fields.contains(MTGFR_INTENT_ACCEPTED));
        assert!(observed.fields.contains(MTGFR_USER_ID));
        assert!(!observed.fields.contains("table_id"));
        assert!(!observed.fields.contains("intent.kind"));
        assert!(!observed.fields.contains("accepted"));
    }

    #[test]
    fn span_for_http_request_scrubs_unparseable_path_from_rpc_method() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        let headers = http::HeaderMap::new();
        let layer = CapturingLayer::default();
        let observed = layer.observed.clone();
        let subscriber = tracing_subscriber::registry().with(layer);
        let garbage_path = "/not-a-valid-grpc-path/with/extra/segments";

        tracing::subscriber::with_default(subscriber, || {
            let span = span_for_http_request(garbage_path, &headers);
            let _enter = span.enter();
        });

        let observed = observed
            .lock()
            .expect("observed span lock")
            .take()
            .expect("span was created");
        assert_eq!(
            observed.values.get(RPC_SERVICE).map(String::as_str),
            Some("unknown")
        );
        assert_eq!(
            observed.values.get(RPC_METHOD).map(String::as_str),
            Some("unknown")
        );
        assert!(
            observed
                .values
                .get(RPC_METHOD)
                .is_none_or(|method| method != garbage_path),
            "rpc.method must not contain the raw HTTP path"
        );
    }

    #[test]
    fn response_grpc_status_is_recorded_on_current_span() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        let headers = http::HeaderMap::new();
        let mut response_headers = http::HeaderMap::new();
        response_headers.insert("grpc-status", "7".parse().unwrap());
        let layer = CapturingLayer::default();
        let observed = layer.observed.clone();
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = span_for_http_request("/mtgfr.v1.Game/SubmitIntent", &headers);
            let _enter = span.enter();
            record_grpc_status_from_headers(&response_headers);
        });

        let observed = observed
            .lock()
            .expect("observed span lock")
            .take()
            .expect("span was created");
        assert_eq!(
            observed
                .values
                .get(RPC_GRPC_STATUS_CODE)
                .map(String::as_str),
            Some("7")
        );
    }

    #[test]
    fn missing_response_grpc_status_records_ok() {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
        let headers = http::HeaderMap::new();
        let response_headers = http::HeaderMap::new();
        let layer = CapturingLayer::default();
        let observed = layer.observed.clone();
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let span = span_for_http_request("/mtgfr.v1.Game/SubmitIntent", &headers);
            let _enter = span.enter();
            record_grpc_status_from_headers(&response_headers);
        });

        let observed = observed
            .lock()
            .expect("observed span lock")
            .take()
            .expect("span was created");
        assert_eq!(
            observed
                .values
                .get(RPC_GRPC_STATUS_CODE)
                .map(String::as_str),
            Some("0")
        );
    }
}
