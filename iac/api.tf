# N versioned API Deployments (1 active, many draining). Each release is its own Deployment+Service
# with INSTANCE_ID = map key (e.g. edh-api-1-2-3). Never rewrite a draining peer's image — that would
# restart the pod and wipe in-memory tables (ADR 0021). Live drain is POST /admin/drain only.
#
# Cookie sticky lives in the SolidStart BFF (`API_UPSTREAMS` / `API_ACTIVE_INSTANCE_ID` on edh-web),
# not nginx.

locals {
  api_env_common = {
    HOST          = "0.0.0.0"
    PORT          = "8080"
    COOKIE_SECURE = "true"
    COOKIE_DOMAIN = var.cookie_domain
    CORS_ORIGIN   = var.cors_origin
    RUST_LOG      = var.log_level
  }

  api_active_image = var.api_instances[var.api_active_instance_id].image
}

check "api_instances_nonempty" {
  assert {
    condition     = length(var.api_instances) > 0
    error_message = "api_instances must contain at least the active instance."
  }
}

check "api_instances_cap" {
  assert {
    condition     = length(var.api_instances) <= var.api_max_instances
    error_message = "api_instances length exceeds api_max_instances (${var.api_max_instances})."
  }
}

check "api_active_in_map" {
  assert {
    condition     = contains(keys(var.api_instances), var.api_active_instance_id)
    error_message = "api_active_instance_id must be a key in api_instances."
  }
}

resource "kubernetes_deployment_v1" "edh_api" {
  for_each = var.api_instances

  wait_for_rollout = true

  metadata {
    name      = each.key
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = each.key
      "mtgfr.io/component" = "api"
      "mtgfr.io/api-role"  = each.key == var.api_active_instance_id ? "active" : "drain"
    })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = each.key }
    }

    template {
      metadata {
        labels = merge(local.common_labels, {
          app                  = each.key
          "mtgfr.io/component" = "api"
        })
      }

      spec {
        container {
          name              = "edh-api"
          image             = each.value.image
          image_pull_policy = "Always"

          port {
            container_port = 8080
          }

          dynamic "env" {
            for_each = merge(local.api_env_common, {
              INSTANCE_ID = each.key
              # Startup default only — live drain via POST /admin/drain (never flip this to restart).
              DRAIN   = "false"
              VERSION = each.value.image
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

  depends_on = [kubernetes_job_v1.edh_migrate]
}

resource "kubernetes_service_v1" "edh_api" {
  for_each = var.api_instances

  wait_for_load_balancer = false

  metadata {
    name      = each.key
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = each.key
      "mtgfr.io/component" = "api"
    })
  }

  spec {
    selector = { app = each.key }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}
