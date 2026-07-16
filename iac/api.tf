# One desired API Deployment for server_image (active). Old pods linger as Terminating while
# the process drains on SIGTERM (distroless — no preStop shell). BFF routes in-game via
# Postgres table_routes → pod DNS on the headless Service (publishNotReadyAddresses).
# New tables only hit Service edh-api (mtgfr.io/api-role=active).

locals {
  api_env_common = {
    HOST          = "0.0.0.0"
    PORT          = "8080"
    COOKIE_SECURE = "true"
    COOKIE_DOMAIN = var.cookie_domain
    CORS_ORIGIN   = var.cors_origin
    RUST_LOG      = var.log_level
  }

  server_image_tag = element(split(":", var.server_image), length(split(":", var.server_image)) - 1)
  api_active_instance_id = format(
    "edh-api-%s",
    trimsuffix(trimprefix(replace(lower(local.server_image_tag), "/[^a-z0-9]+/", "-"), "-"), "-")
  )

  api_active_image     = var.server_image
  api_headless_service = "edh-api-headless"
}

resource "kubernetes_deployment_v1" "edh_api" {
  wait_for_rollout = true

  metadata {
    name      = local.api_active_instance_id
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = local.api_active_instance_id
      "mtgfr.io/component" = "api"
      "mtgfr.io/api-role"  = "active"
    })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = local.api_active_instance_id }
    }

    template {
      metadata {
        labels = merge(local.common_labels, {
          app                  = local.api_active_instance_id
          "mtgfr.io/component" = "api"
          "mtgfr.io/api-role"  = "active"
        })
      }

      spec {
        termination_grace_period_seconds = var.api_termination_grace_seconds

        container {
          name              = "edh-api"
          image             = var.server_image
          image_pull_policy = "Always"

          port {
            container_port = 8080
          }

          dynamic "env" {
            for_each = merge(local.api_env_common, {
              INSTANCE_ID = local.api_active_instance_id
              DRAIN       = "false"
              VERSION     = var.server_image
            })
            content {
              name  = env.key
              value = env.value
            }
          }

          env {
            name = "POD_NAME"
            value_from {
              field_ref {
                field_path = "metadata.name"
              }
            }
          }

          env {
            name = "POD_NAMESPACE"
            value_from {
              field_ref {
                field_path = "metadata.namespace"
              }
            }
          }

          # k8s expands $(VAR) from earlier env entries in the same container.
          env {
            name  = "POD_DNS"
            value = "$(POD_NAME).${local.api_headless_service}.$(POD_NAMESPACE).svc.cluster.local"
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

# Newest-only: new tables / lobby Start seed.
resource "kubernetes_service_v1" "edh_api_active" {
  wait_for_load_balancer = false

  metadata {
    name      = "edh-api"
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = "edh-api"
      "mtgfr.io/component" = "api"
    })
  }

  spec {
    selector = {
      "mtgfr.io/component" = "api"
      "mtgfr.io/api-role"  = "active"
    }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}

# Sticky dial for Terminating pods (BFF uses pod DNS from table_routes).
resource "kubernetes_service_v1" "edh_api_headless" {
  wait_for_load_balancer = false

  metadata {
    name      = local.api_headless_service
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = local.api_headless_service
      "mtgfr.io/component" = "api"
    })
  }

  spec {
    cluster_ip                  = "None"
    publish_not_ready_addresses = true
    selector = {
      "mtgfr.io/component" = "api"
    }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}
