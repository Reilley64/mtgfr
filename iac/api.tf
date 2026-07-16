# Deploy PRD §Rolling deployment model / §Table-instance affinity. Two peer Deployments —
# `edh-api` (active) and `edh-api-drain` (present only mid-roll) — each with a stable INSTANCE_ID
# env var (not the pod name; pod names change on restart/reschedule and would invalidate sticky
# cookies). `edh-proxy` (proxy.tf) routes on the `mtgfr-instance` cookie between them.
#
# `DRAIN` here is a *startup* default only — the live drain toggle is `POST /admin/drain` against
# the already-running process (deploy PRD §Rolling deployment model step 2). Flipping this env var
# on an existing Deployment and letting Kubernetes restart the pod would wipe its in-memory tables
# (ADR 0021) — do not use Terraform/env changes as the drain mechanism.

locals {
  api_env_common = {
    HOST          = "0.0.0.0"
    PORT          = "8080"
    COOKIE_SECURE = "true"
    COOKIE_DOMAIN = var.cookie_domain
    CORS_ORIGIN   = var.cors_origin
    RUST_LOG      = var.log_level
  }
}

resource "kubernetes_deployment_v1" "edh_api" {
  wait_for_rollout = true

  metadata {
    name      = "edh-api"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api" })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = "edh-api" }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-api" })
      }

      spec {
        container {
          name              = "edh-api"
          image             = var.server_image
          image_pull_policy = "Always"

          port {
            container_port = 8080
          }

          dynamic "env" {
            for_each = merge(local.api_env_common, {
              INSTANCE_ID = "edh-api"
              DRAIN       = "false"
              VERSION     = var.server_image
            })
            content {
              name  = env.key
              value = env.value
            }
          }

          env {
            name = "DATABASE_URL"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_db.metadata[0].name
                key  = "DATABASE_URL"
              }
            }
          }

          env {
            name = "ADMIN_TOKEN"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_admin.metadata[0].name
                key  = "ADMIN_TOKEN"
              }
            }
          }

          # Health endpoints per deploy PRD §Server changes required. `/health/drain` and
          # `/admin/drain` are cluster-internal only (network-policy.tf) — probes here hit them
          # from inside the pod's own namespace, which NetworkPolicy always allows.
          liveness_probe {
            http_get {
              path = "/health/live"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }

          readiness_probe {
            http_get {
              path = "/health/ready"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }
        }
      }
    }
  }

  depends_on = [kubernetes_job_v1.edh_migrate]
}

resource "kubernetes_service_v1" "edh_api" {
  wait_for_load_balancer = false

  metadata {
    name      = "edh-api"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api" })
  }

  spec {
    selector = { app = "edh-api" }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}

# Present only during a roll (deploy PRD target topology: "Deployment edh-api-drain (serve;
# during rolls only)"). Bring up with api_drain_enabled = true and server_image_drain pinned to
# the outgoing release; tear down (api_drain_enabled = false) once GET /health/drain on this peer
# reports active_tables = 0 (scripts/wait-drain.sh via kubectl port-forward — never the public
# tunnel URL).
resource "kubernetes_deployment_v1" "edh_api_drain" {
  count = var.api_drain_enabled ? 1 : 0

  wait_for_rollout = true

  metadata {
    name      = "edh-api-drain"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api-drain" })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = "edh-api-drain" }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-api-drain" })
      }

      spec {
        container {
          name              = "edh-api-drain"
          image             = var.server_image_drain
          image_pull_policy = "Always"

          port {
            container_port = 8080
          }

          dynamic "env" {
            for_each = merge(local.api_env_common, {
              INSTANCE_ID = "edh-api-drain"
              DRAIN       = "false" # startup default; the outgoing instance is flipped live via POST /admin/drain, not by rolling this Deployment
              VERSION     = var.server_image_drain
            })
            content {
              name  = env.key
              value = env.value
            }
          }

          env {
            name = "DATABASE_URL"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_db.metadata[0].name
                key  = "DATABASE_URL"
              }
            }
          }

          env {
            name = "ADMIN_TOKEN"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_admin.metadata[0].name
                key  = "ADMIN_TOKEN"
              }
            }
          }

          liveness_probe {
            http_get {
              path = "/health/live"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }

          readiness_probe {
            http_get {
              path = "/health/ready"
              port = 8080
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "edh_api_drain" {
  count = var.api_drain_enabled ? 1 : 0

  wait_for_load_balancer = false

  metadata {
    name      = "edh-api-drain"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api-drain" })
  }

  spec {
    selector = { app = "edh-api-drain" }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}
