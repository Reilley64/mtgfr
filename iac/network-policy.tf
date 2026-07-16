# Deploy PRD §Decisions (locked) — "Admin / drain endpoints: Not public (NetworkPolicy blocks
# tunnel); apply machine uses kubectl port-forward". NetworkPolicy filters by pod/port (L3/L4), not
# HTTP path, so the effective control here is narrower and stronger than just /admin* + drain: the
# `cloudflared` pods (the only route from the public internet) are only ever allowed to reach
# `edh-web` and `edh-api-proxy` — never `edh-api` / `edh-api-drain` directly. The L7 half (nginx
# 404s `/admin/*` and `/health/drain`) lives in api-proxy.tf.
#
# `kubectl port-forward` from the apply machine goes through the kubelet directly into the pod's
# network namespace rather than over the pod's normal CNI-managed ingress path, so it is not
# subject to these Ingress NetworkPolicies on most CNIs — consistent with the deploy PRD's
# assumption that port-forward is the one path allowed to reach /admin and /health/drain.

resource "kubernetes_network_policy" "edh_api_ingress" {
  metadata {
    name      = "edh-api-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_expressions {
        key      = "app"
        operator = "In"
        values   = ["edh-api", "edh-api-drain"]
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "edh-api-proxy" }
        }
      }

      ports {
        port     = "8080"
        protocol = "TCP"
      }
    }

    policy_types = ["Ingress"]
  }
}

resource "kubernetes_network_policy" "edh_api_proxy_ingress" {
  metadata {
    name      = "edh-api-proxy-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = { app = "edh-api-proxy" }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "cloudflared" }
        }
      }

      ports {
        port     = "8080"
        protocol = "TCP"
      }
    }

    policy_types = ["Ingress"]
  }
}

resource "kubernetes_network_policy" "edh_web_ingress" {
  metadata {
    name      = "edh-web-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = { app = "edh-web" }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "cloudflared" }
        }
      }

      ports {
        port     = "8080"
        protocol = "TCP"
      }
    }

    policy_types = ["Ingress"]
  }
}

# Bitnami's postgresql chart (postgres.tf) labels the primary pod
# app.kubernetes.io/name=postgresql, app.kubernetes.io/instance=<release name> — the release name
# is local.postgres_service ("postgres", via fullnameOverride). Only the three workloads that
# actually hold DATABASE_URL may reach it: edh-api, edh-api-drain (api.tf), and edh-migrate
# (migrate.tf, the `app` label on its Job's pod template). Egress from those pods is unrestricted
# (no NetworkPolicy selects them for egress), so this ingress-only rule is the full control.
resource "kubernetes_network_policy" "postgres_ingress" {
  metadata {
    name      = "postgres-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = {
        "app.kubernetes.io/name"     = "postgresql"
        "app.kubernetes.io/instance" = local.postgres_service
      }
    }

    ingress {
      from {
        pod_selector {
          match_expressions {
            key      = "app"
            operator = "In"
            values   = ["edh-api", "edh-api-drain", "edh-migrate"]
          }
        }
      }

      ports {
        port     = "5432"
        protocol = "TCP"
      }
    }

    policy_types = ["Ingress"]
  }
}
