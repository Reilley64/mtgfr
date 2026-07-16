# Deploy PRD §Decisions (locked) — "/health/drain: Not public (NetworkPolicy blocks tunnel);
# apply machine uses kubectl port-forward". NetworkPolicy filters by pod/port (L3/L4), not
# HTTP path: `cloudflared` may only reach `edh-web`. `edh-web` (SolidStart BFF) may reach all API
# pods (`mtgfr.io/component=api`). `/health/drain` is 404'd by the BFF and remains port-forward only.
#
# `kubectl port-forward` from the apply machine goes through the kubelet directly into the pod's
# network namespace rather than over the pod's normal CNI-managed ingress path, so it is not
# subject to these Ingress NetworkPolicies on most CNIs — consistent with the deploy PRD's
# assumption that port-forward is the one path allowed to reach /health/drain.

resource "kubernetes_network_policy_v1" "edh_api_ingress" {
  metadata {
    name      = "edh-api-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = {
        "mtgfr.io/component" = "api"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "edh-web" }
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

resource "kubernetes_network_policy_v1" "edh_web_ingress" {
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

# postgres.tf labels the primary pod `app=postgres`. API, Toasty migrate, BFF, and web DB
# jobs may reach it (mtgfr + mtgfr_web). Egress from those pods is unrestricted.
resource "kubernetes_network_policy_v1" "postgres_ingress" {
  metadata {
    name      = "postgres-ingress"
    namespace = local.namespace
  }

  spec {
    pod_selector {
      match_labels = {
        app = local.postgres_service
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = {
            "mtgfr.io/component" = "api"
          }
        }
      }

      ports {
        port     = "5432"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "edh-migrate" }
        }
      }

      ports {
        port     = "5432"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "edh-web" }
        }
      }

      ports {
        port     = "5432"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "edh-web-migrate" }
        }
      }

      ports {
        port     = "5432"
        protocol = "TCP"
      }
    }

    ingress {
      from {
        pod_selector {
          match_labels = { app = "postgres-create-web-db" }
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
