//! OpenTelemetry + `tracing` bootstrap for the API process.
//!
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is set, exports traces/metrics/logs over OTLP/HTTP.
//! Otherwise installs a fmt subscriber only (honors `RUST_LOG`). Local/dev never fails on export.
//! Exporter build failures soft-fall back to fmt-only so a down Alloy does not crash the API.
//!
//! Batch export runs on the Tokio runtime (`install_batch(Tokio)` pattern — see LogRocket's
//! "Composing the underpinnings of an observable Rust application"), so the async HTTP client
//! has a reactor. Call [`init`] from inside a Tokio runtime (e.g. `#[tokio::main]`).

use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::log_processor_with_async_runtime::BatchLogProcessor;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::periodic_reader_with_async_runtime::PeriodicReader;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
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

fn env_nonempty(key: &str) -> Option<String> {
    let raw = std::env::var(key).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Tag from a GHCR image ref (`ghcr.io/o/mtgfr-server:2.3.0` → `2.3.0`), else the raw value.
fn image_tag(version_or_image: &str) -> String {
    version_or_image
        .rsplit_once(':')
        .map(|(_, tag)| tag.to_string())
        .unwrap_or_else(|| version_or_image.to_string())
}

/// Prefer bake-time `APP_VERSION`; fall back to the image tag in `VERSION`.
fn service_version() -> Option<String> {
    env_nonempty("APP_VERSION").or_else(|| env_nonempty("VERSION").map(|v| image_tag(&v)))
}

fn resource_attributes() -> Vec<KeyValue> {
    let mut attrs = vec![KeyValue::new("service.name", service_name())];
    if let Some(version) = service_version() {
        attrs.push(KeyValue::new("service.version", version));
    }
    if let Some(commit) = env_nonempty("GIT_COMMIT") {
        // OTEL VCS semconv — full git SHA from the image build.
        attrs.push(KeyValue::new("vcs.ref.head.revision", commit));
    }
    if let Some(instance_id) = env_nonempty("INSTANCE_ID") {
        attrs.push(KeyValue::new("service.instance.id", instance_id));
    }
    attrs
}

fn resource() -> Resource {
    Resource::builder_empty()
        .with_attributes(resource_attributes())
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

/// Build OTLP providers with Tokio-backed batch processors (LogRocket `install_batch(Tokio)`).
///
/// Must run on a Tokio runtime so the processors can spawn export tasks.
fn try_build_otlp(endpoint: &str) -> Result<OtlpStack, String> {
    let resource = resource();

    // Programmatic `with_endpoint` is used as-is (no path append) — include signal paths.
    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/traces"))
        .build()
        .map_err(|e| format!("span exporter: {e}"))?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(span_exporter, runtime::Tokio).build())
        .with_resource(resource.clone())
        .build();

    let metric_exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .build()
        .map_err(|e| format!("metric exporter: {e}"))?;
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(metric_exporter, runtime::Tokio).build())
        .with_resource(resource.clone())
        .build();

    let log_exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/logs"))
        .build()
        .map_err(|e| format!("log exporter: {e}"))?;
    let logger_provider = SdkLoggerProvider::builder()
        .with_log_processor(BatchLogProcessor::builder(log_exporter, runtime::Tokio).build())
        .with_resource(resource)
        .build();

    Ok(OtlpStack {
        tracer_provider,
        meter_provider,
        logger_provider,
    })
}

/// Install the global tracing subscriber. Safe to call once; later calls are no-ops.
///
/// When OTLP is enabled, must be called from a Tokio runtime context.
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

    #[test]
    fn image_tag_strips_registry_ref() {
        assert_eq!(
            image_tag("ghcr.io/reilley64/mtgfr-server:2.3.0"),
            "2.3.0"
        );
        assert_eq!(image_tag("2.3.0"), "2.3.0");
    }

    /// Regression: Tokio batch processors + async HTTP client must build on a runtime
    /// (thread-based batch + async reqwest panics with "no reactor running").
    #[tokio::test(flavor = "multi_thread")]
    async fn try_build_otlp_builds_http_exporters() {
        let stack = try_build_otlp("http://127.0.0.1:4318")
            .unwrap_or_else(|e| panic!("expected OTLP HTTP exporters to build, got {e}"));
        // Batch exporters spawn workers; shut down so the test process exits.
        let _ = stack.tracer_provider.shutdown();
        let _ = stack.meter_provider.shutdown();
        let _ = stack.logger_provider.shutdown();
    }
}
