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
        logs   = [loki.write.default.receiver]
        traces = [otelcol.processor.batch.default.input]
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
  version    = "6.55.0"
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
  version    = "29.17.0"
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
  version    = "1.10.1"
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
