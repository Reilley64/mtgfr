# Self-hosted LGTM (Loki + Grafana + Tempo + Prometheus) + Alloy collector.
# Operator UI via kubectl port-forward only — no Cloudflare Tunnel hostname (ADR 0034).

resource "kubernetes_namespace_v1" "observability" {
  metadata {
    name = var.namespace_observability
    labels = merge(local.common_labels, {
      "app.kubernetes.io/name" = "observability"
    })
  }
}

resource "random_password" "grafana_admin" {
  length  = 32
  special = false
}

resource "kubernetes_secret_v1" "grafana_admin" {
  wait_for_service_account_token = false

  metadata {
    name      = "grafana-admin"
    namespace = local.observability_namespace
  }

  data = {
    admin-user     = "admin"
    admin-password = random_password.grafana_admin.result
  }

  type = "Opaque"
}

locals {
  observability_namespace = kubernetes_namespace_v1.observability.metadata[0].name

  # Predictable ClusterDNS for app env vars and Alloy exporters.
  alloy_otlp_http = "http://alloy.${local.observability_namespace}.svc:4318"
  alloy_faro      = "http://alloy.${local.observability_namespace}.svc:12347/collect"
  loki_push       = "http://loki-gateway.${local.observability_namespace}.svc/loki/api/v1/push"
  tempo_otlp      = "tempo.${local.observability_namespace}.svc:4317"
  prometheus_rw   = "http://prometheus-server.${local.observability_namespace}.svc/api/v1/write"

  alloy_config = <<-EOT
    logging {
      level  = "info"
      format = "logfmt"
    }

    otelcol.receiver.otlp "default" {
      grpc {
        endpoint = "0.0.0.0:4317"
      }

      http {
        endpoint = "0.0.0.0:4318"
      }

      output {
        metrics = [otelcol.processor.batch.default.input]
        logs    = [otelcol.processor.batch.default.input]
        traces  = [otelcol.processor.batch.default.input]
      }
    }

    faro.receiver "default" {
      server {
        listen_address           = "0.0.0.0"
        listen_port              = 12347
        // BFF proxies same-origin; no browser CORS on Alloy.
        max_allowed_payload_size = "512KiB"
        rate_limiting {
          enabled = true
          rate    = 100
        }
      }

      output {
        logs   = [loki.process.faro_web_vitals.receiver]
        traces = [otelcol.processor.batch.default.input]
      }
    }

    // faro.receiver has no metrics output — web vitals arrive as logfmt measurement lines
    // (`kind=measurement type=web-vitals lcp=1234.500000 ...`). Lines still reach Loki unchanged;
    // the metric stages mirror the values into histograms so p75 panels are a Prometheus query
    // instead of a LogQL unwrap, and survive past Loki's 7d retention.
    loki.process "faro_web_vitals" {
      forward_to = [loki.write.default.receiver]

      stage.logfmt {
        mapping = {
          cls  = "",
          fcp  = "",
          inp  = "",
          lcp  = "",
          ttfb = "",
        }
      }

      // Keyed on the value name, not on kind/type: only web-vitals measurements carry these as
      // top-level keys, and `stage.match` with action=keep would drop every other Faro line
      // (errors, events, other measurements) out of Loki. A metric stage whose source key is
      // absent from the line is skipped.
      //
      // Timings are milliseconds as Faro reports them (no _seconds rename — converting needs a
      // template stage that would then have to cope with lines missing the key). Shared buckets
      // cover every Google threshold in play: INP 200/500, TTFB 800/1800, FCP 1800/3000,
      // LCP 2500/4000.
      stage.metrics {
        metric.histogram {
          name              = "web_vitals_cls"
          prefix            = "faro_"
          description       = "Faro browser Cumulative Layout Shift (unitless)."
          source            = "cls"
          buckets           = [0.01, 0.05, 0.1, 0.15, 0.25, 0.5, 1]
          max_idle_duration = "24h"
        }

        metric.histogram {
          name              = "web_vitals_fcp_milliseconds"
          prefix            = "faro_"
          description       = "Faro browser First Contentful Paint."
          source            = "fcp"
          buckets           = [50, 100, 200, 300, 500, 800, 1000, 1800, 2500, 3000, 4000, 6000, 10000]
          max_idle_duration = "24h"
        }

        metric.histogram {
          name              = "web_vitals_inp_milliseconds"
          prefix            = "faro_"
          description       = "Faro browser Interaction to Next Paint."
          source            = "inp"
          buckets           = [50, 100, 200, 300, 500, 800, 1000, 1800, 2500, 3000, 4000, 6000, 10000]
          max_idle_duration = "24h"
        }

        metric.histogram {
          name              = "web_vitals_lcp_milliseconds"
          prefix            = "faro_"
          description       = "Faro browser Largest Contentful Paint."
          source            = "lcp"
          buckets           = [50, 100, 200, 300, 500, 800, 1000, 1800, 2500, 3000, 4000, 6000, 10000]
          max_idle_duration = "24h"
        }

        metric.histogram {
          name              = "web_vitals_ttfb_milliseconds"
          prefix            = "faro_"
          description       = "Faro browser Time To First Byte."
          source            = "ttfb"
          buckets           = [50, 100, 200, 300, 500, 800, 1000, 1800, 2500, 3000, 4000, 6000, 10000]
          max_idle_duration = "24h"
        }
      }
    }

    // stage.metrics only exposes series on Alloy's own /metrics — nothing forwards them. Scrape
    // ourselves over loopback (exempt from the alloy-ingress NetworkPolicy) and keep the faro_
    // series only, so Alloy's several hundred internal series stay out of Prometheus.
    prometheus.scrape "self" {
      targets         = [{ __address__ = "127.0.0.1:12345" }]
      job_name        = "alloy"
      scrape_interval = "60s"
      forward_to      = [prometheus.relabel.faro_only.receiver]
    }

    prometheus.relabel "faro_only" {
      forward_to = [prometheus.remote_write.default.receiver]

      rule {
        source_labels = ["__name__"]
        regex         = "faro_web_vitals_.*"
        action        = "keep"
      }
    }

    otelcol.processor.batch "default" {
      output {
        metrics = [otelcol.exporter.prometheus.default.input]
        logs    = [otelcol.exporter.loki.default.input]
        traces  = [otelcol.exporter.otlp.tempo.input]
      }
    }

    otelcol.exporter.otlp "tempo" {
      client {
        endpoint = "${local.tempo_otlp}"
        tls {
          insecure = true
        }
      }
    }

    otelcol.exporter.loki "default" {
      forward_to = [loki.write.default.receiver]
    }

    loki.write "default" {
      endpoint {
        url = "${local.loki_push}"
      }
    }

    otelcol.exporter.prometheus "default" {
      forward_to = [prometheus.remote_write.default.receiver]
    }

    prometheus.remote_write "default" {
      endpoint {
        url = "${local.prometheus_rw}"
      }
    }
  EOT
}

# ── Loki (SingleBinary, filesystem, 7d) ─────────────────────────────────────────────────────────

resource "helm_release" "loki" {
  name       = "loki"
  repository = "https://grafana.github.io/helm-charts"
  chart      = "loki"
  version    = "7.1.0"
  namespace  = local.observability_namespace

  wait    = true
  timeout = 600

  values = [
    yamlencode({
      deploymentMode = "SingleBinary"
      loki = {
        auth_enabled = false
        commonConfig = {
          replication_factor = 1
        }
        storage = {
          type = "filesystem"
        }
        schemaConfig = {
          configs = [{
            from         = "2024-01-01"
            store        = "tsdb"
            object_store = "filesystem"
            schema       = "v13"
            index = {
              prefix = "loki_index_"
              period = "24h"
            }
          }]
        }
        limits_config = {
          retention_period          = "168h"
          allow_structured_metadata = true
        }
        compactor = {
          retention_enabled    = true
          delete_request_store = "filesystem"
        }
      }
      singleBinary = {
        replicas = 1
        persistence = {
          enabled = true
          size    = var.observability_storage_size
        }
      }
      backend      = { replicas = 0 }
      read         = { replicas = 0 }
      write        = { replicas = 0 }
      gateway      = { enabled = true }
      chunksCache  = { enabled = false }
      resultsCache = { enabled = false }
      lokiCanary   = { enabled = false }
      test         = { enabled = false }
      minio        = { enabled = false }
    })
  ]

  depends_on = [kubernetes_namespace_v1.observability]
}

# ── Tempo (monolithic, 7d) ──────────────────────────────────────────────────────────────────────

resource "helm_release" "tempo" {
  name       = "tempo"
  repository = "https://grafana.github.io/helm-charts"
  chart      = "tempo"
  version    = "1.24.4"
  namespace  = local.observability_namespace

  wait    = true
  timeout = 600

  values = [
    yamlencode({
      tempo = {
        retention = "168h"
        receivers = {
          otlp = {
            protocols = {
              grpc = {
                endpoint = "0.0.0.0:4317"
              }
              http = {
                endpoint = "0.0.0.0:4318"
              }
            }
          }
        }
        # TraceQL metrics (`{...} | rate()`, what Grafana Drilldown > Traces issues) are served
        # for the recent window by the metrics-generator. Without a `metrics_generator.storage.path`
        # Tempo logs "metrics-generator is not configured", never joins the generator ring, and the
        # query 500s with "error finding generators: empty ring". Paths live on the PVC (/var/tempo).
        metricsGenerator = {
          enabled        = true
          remoteWriteUrl = "${local.prometheus_rw}"
          processor = {
            local_blocks = {
              # Drilldown counts client spans too, not just server spans.
              filter_server_spans = false
              # Flush RF1 blocks so metrics queries reach past the live window.
              flush_to_storage = true
            }
          }
          storage        = { path = "/var/tempo/generator/wal" }
          traces_storage = { path = "/var/tempo/generator/traces" }
        }
        # The generator only runs a processor a tenant asks for; local-blocks backs TraceQL metrics.
        # ponytail: local-blocks only — add service-graphs/span-metrics when the service map or
        # tracesToMetrics panels are actually wanted (they remote-write series to Prometheus).
        overrides = {
          defaults = {
            metrics_generator = {
              processors = ["local-blocks"]
            }
          }
        }
      }
      persistence = {
        enabled = true
        size    = var.observability_storage_size
      }
    })
  ]

  depends_on = [kubernetes_namespace_v1.observability]
}

# ── Prometheus (app metrics sink only, 15d) ─────────────────────────────────────────────────────

resource "helm_release" "prometheus" {
  name       = "prometheus"
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "prometheus"
  version    = "29.19.0"
  namespace  = local.observability_namespace

  wait    = true
  timeout = 600

  values = [
    yamlencode({
      alertmanager               = { enabled = false }
      "kube-state-metrics"       = { enabled = false }
      "prometheus-node-exporter" = { enabled = false }
      "prometheus-pushgateway"   = { enabled = false }
      server = {
        retention = "15d"
        persistentVolume = {
          enabled = true
          size    = var.observability_storage_size
        }
        # Accept remote_write from Alloy (OTLP metrics → Prometheus).
        extraArgs = {
          "web.enable-remote-write-receiver" = null
        }
      }
    })
  ]

  depends_on = [kubernetes_namespace_v1.observability]
}

# ── Grafana (port-forward only) ─────────────────────────────────────────────────────────────────

# Helm resolves `chart` as a local path before consulting `repository`, so a directory named
# `grafana/` next to this file would shadow the remote chart ("Chart.yaml file is missing").
# Dashboards live in `dashboards/` for that reason — do not name a local dir after a chart.
resource "helm_release" "grafana" {
  name       = "grafana"
  repository = "https://grafana.github.io/helm-charts"
  chart      = "grafana"
  version    = "10.5.15"
  namespace  = local.observability_namespace

  wait    = true
  timeout = 600

  values = [
    yamlencode({
      admin = {
        existingSecret = kubernetes_secret_v1.grafana_admin.metadata[0].name
        userKey        = "admin-user"
        passwordKey    = "admin-password"
      }
      persistence = {
        enabled = true
        size    = "2Gi"
      }
      # PVC dirs (csv/png/pdf) are mode 0700 as uid 472. The chart's init-chown
      # runs as root with only CAP_CHOWN (no DAC_OVERRIDE), so chown -R fails with
      # Permission denied and the rollout never becomes Ready. Data is already
      # owned correctly after first start — skip the init.
      initChownData = {
        enabled = false
      }
      # Do NOT set plugins = ["grafana-faro-app"]: that ID 404s on grafana.com and
      # Grafana 12's background installer treats install failure as fatal (crash loop).
      # Faro RUM for self-hosted LGTM is Loki (events) + Tempo (traces) dashboards —
      # the Cloud "Frontend Observability" app is not available as an OSS plugin.
      datasources = {
        "datasources.yaml" = {
          apiVersion = 1
          datasources = [
            {
              name      = "Prometheus"
              type      = "prometheus"
              uid       = "prometheus"
              url       = "http://prometheus-server.${local.observability_namespace}.svc"
              access    = "proxy"
              isDefault = true
            },
            {
              name   = "Loki"
              type   = "loki"
              uid    = "loki"
              url    = "http://loki-gateway.${local.observability_namespace}.svc"
              access = "proxy"
              jsonData = {
                derivedFields = [{
                  datasourceUid = "tempo"
                  matcherRegex  = "\"trace_id\":\"(\\w+)\""
                  name          = "TraceID"
                  url           = "$${__value.raw}"
                }]
              }
            },
            {
              name   = "Tempo"
              type   = "tempo"
              uid    = "tempo"
              url    = "http://tempo.${local.observability_namespace}.svc:3200"
              access = "proxy"
              jsonData = {
                tracesToLogsV2 = {
                  datasourceUid   = "loki"
                  filterByTraceID = true
                }
                tracesToMetrics = {
                  datasourceUid = "prometheus"
                }
                serviceMap = {
                  datasourceUid = "prometheus"
                }
                lokiSearch = {
                  datasourceUid = "loki"
                }
              }
            },
          ]
        }
      }
      dashboardProviders = {
        "dashboardproviders.yaml" = {
          apiVersion = 1
          providers = [{
            name            = "mtgfr"
            orgId           = 1
            folder          = "mtgfr"
            type            = "file"
            disableDeletion = false
            editable        = true
            options = {
              path = "/var/lib/grafana/dashboards/mtgfr"
            }
          }]
        }
      }
      dashboards = {
        mtgfr = {
          "mtgfr-otel-red" = {
            json = file("${path.module}/dashboards/mtgfr-otel-red.json")
          }
        }
      }
    })
  ]

  depends_on = [
    kubernetes_secret_v1.grafana_admin,
    helm_release.loki,
    helm_release.tempo,
    helm_release.prometheus,
  ]
}

# ── Alloy (OTLP + Faro → LGTM) ──────────────────────────────────────────────────────────────────

resource "helm_release" "alloy" {
  name       = "alloy"
  repository = "https://grafana.github.io/helm-charts"
  chart      = "alloy"
  version    = "1.11.0"
  namespace  = local.observability_namespace

  wait    = true
  timeout = 600

  values = [
    yamlencode({
      controller = {
        type     = "deployment"
        replicas = 1
      }
      alloy = {
        stabilityLevel = "generally-available"
        # Pinned because `prometheus.scrape "self"` dials 127.0.0.1 on this port for the
        # Faro web-vitals histograms; a chart-default change would silently stop that scrape.
        listenPort = 12345
        configMap = {
          content = local.alloy_config
        }
        extraPorts = [
          {
            name       = "otlp-grpc"
            port       = 4317
            targetPort = 4317
            protocol   = "TCP"
          },
          {
            name       = "otlp-http"
            port       = 4318
            targetPort = 4318
            protocol   = "TCP"
          },
          {
            name       = "faro"
            port       = 12347
            targetPort = 12347
            protocol   = "TCP"
          },
        ]
      }
      service = {
        enabled = true
        type    = "ClusterIP"
      }
    })
  ]

  depends_on = [
    helm_release.loki,
    helm_release.tempo,
    helm_release.prometheus,
  ]
}

# Cross-namespace ingress: edh-web (Faro + OTLP) and edh-api (OTLP) → Alloy.
resource "kubernetes_network_policy_v1" "alloy_ingress" {
  metadata {
    name      = "alloy-ingress"
    namespace = local.observability_namespace
  }

  spec {
    pod_selector {
      match_labels = {
        "app.kubernetes.io/name" = "alloy"
      }
    }

    ingress {
      from {
        namespace_selector {
          match_labels = {
            "kubernetes.io/metadata.name" = local.namespace
          }
        }
        pod_selector {
          match_labels = { app = "edh-web" }
        }
      }

      ports {
        port     = "4317"
        protocol = "TCP"
      }
      ports {
        port     = "4318"
        protocol = "TCP"
      }
      ports {
        port     = "12347"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        namespace_selector {
          match_labels = {
            "kubernetes.io/metadata.name" = local.namespace
          }
        }
        pod_selector {
          match_labels = {
            "mtgfr.io/component" = "api"
          }
        }
      }

      ports {
        port     = "4317"
        protocol = "TCP"
      }
      ports {
        port     = "4318"
        protocol = "TCP"
      }
    }

    policy_types = ["Ingress"]
  }

  depends_on = [helm_release.alloy]
}
