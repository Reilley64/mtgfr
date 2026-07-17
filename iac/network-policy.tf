# `/health/drain` is not public: cloudflared → edh-web only; edh-web → API pods.
# Port-forward (apply machine) bypasses these Ingress policies on most CNIs.

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

      # ADR 0032: the BFF dials the tonic gRPC service on this port for every game/auth/decks/
      # cards call; 8080 above is now health checks only.
      ports {
        port     = "50051"
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
