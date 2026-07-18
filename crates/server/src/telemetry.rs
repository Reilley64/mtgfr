//! OpenTelemetry + `tracing` bootstrap for the API process.
//!
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is set, exports traces/metrics/logs over OTLP/HTTP.
//! Otherwise installs a fmt subscriber only (honors `RUST_LOG`). Local/dev never fails on export.
//! Exporter build failures soft-fall back to fmt-only so a down Alloy does not crash the API.

use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static INIT: OnceLock<()> = OnceLock::new();

fn service_name() -> String {
    std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "edh-api".to_string())
}

fn otlp_endpoint() -> Option<String> {
    let raw = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.trim_end_matches('/').to_string())
}

fn resource() -> Resource {
    Resource::builder_empty()
        .with_attributes([KeyValue::new("service.name", service_name())])
        .build()
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

fn install_fmt_only(filter: EnvFilter) {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

struct OtlpStack {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: SdkLoggerProvider,
}

fn try_build_otlp(endpoint: &str) -> Result<OtlpStack, String> {
    let resource = resource();

    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/traces"))
        .build()
        .map_err(|e| format!("span exporter: {e}"))?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_resource(resource.clone())
        .build();

    let metric_exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .build()
        .map_err(|e| format!("metric exporter: {e}"))?;
    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(resource.clone())
        .build();

    let log_exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/logs"))
        .build()
        .map_err(|e| format!("log exporter: {e}"))?;
    let logger_provider = SdkLoggerProvider::builder()
        .with_batch_exporter(log_exporter)
        .with_resource(resource)
        .build();

    Ok(OtlpStack {
        tracer_provider,
        meter_provider,
        logger_provider,
    })
}

/// Install the global tracing subscriber. Safe to call once; later calls are no-ops.
pub fn init() {
    INIT.get_or_init(|| {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

        let filter = env_filter();
        let Some(endpoint) = otlp_endpoint() else {
            install_fmt_only(filter);
            // Subscriber not ready for tracing:: yet in all paths — use eprintln once.
            eprintln!("telemetry: fmt only (OTEL_EXPORTER_OTLP_ENDPOINT unset)");
            return;
        };

        match try_build_otlp(&endpoint) {
            Ok(stack) => {
                let tracer = stack.tracer_provider.tracer(service_name());
                opentelemetry::global::set_tracer_provider(stack.tracer_provider);
                opentelemetry::global::set_meter_provider(stack.meter_provider);

                let otel_trace = tracing_opentelemetry::layer().with_tracer(tracer);
                let otel_logs = OpenTelemetryTracingBridge::new(&stack.logger_provider);
                let fmt_layer = tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_level(true);

                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .with(otel_trace)
                    .with(otel_logs)
                    .init();

                // Logger provider must outlive the bridge; keep for process lifetime.
                std::mem::forget(stack.logger_provider);
                tracing::info!(%endpoint, "telemetry: OTLP HTTP exporters enabled");
            }
            Err(err) => {
                install_fmt_only(filter);
                tracing::warn!(%endpoint, error = %err, "telemetry: OTLP init failed; fmt only");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init();
        init();
    }

    #[test]
    fn otlp_endpoint_trims_and_rejects_blank() {
        assert_eq!(
            {
                let raw = "  http://alloy:4318/  ";
                let trimmed = raw.trim();
                (!trimmed.is_empty()).then(|| trimmed.trim_end_matches('/').to_string())
            },
            Some("http://alloy:4318".to_string())
        );
        assert_eq!(
            {
                let raw = "   ";
                let trimmed = raw.trim();
                (!trimmed.is_empty()).then(|| trimmed.trim_end_matches('/').to_string())
            },
            None
        );
    }

    /// Regression: default otlp features enable both reqwest clients, which
    /// skips auto-wiring and yields `no http client specified` (empty Tempo).
    #[test]
    fn try_build_otlp_builds_http_exporters() {
        let stack = try_build_otlp("http://127.0.0.1:4318")
            .unwrap_or_else(|e| panic!("expected OTLP HTTP exporters to build, got {e}"));
        // Batch exporters spawn workers; shut down so the test process exits.
        let _ = stack.tracer_provider.shutdown();
        let _ = stack.meter_provider.shutdown();
        let _ = stack.logger_provider.shutdown();
    }
}
