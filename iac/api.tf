# N versioned API Deployments (1 active, many draining). Each release is its own Deployment+Service
# with INSTANCE_ID = map key (e.g. edh-api-1-2-3). Never rewrite a draining peer's image — that would
# restart the pod and wipe in-memory tables (ADR 0021). Live drain is POST /admin/drain only.
#
# Operator sets `server_image` only. Drain peers live in ConfigMap edh-api-peers (kubectl from
# deploy/GC); Terraform refreshes that map and ignores local `data` so bare apply does not wipe peers.
# Cookie sticky lives in the SolidStart BFF on edh-web.

resource "kubernetes_config_map_v1" "edh_api_peers" {
  metadata {
    name      = "edh-api-peers"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api-peers" })
  }

  # Bootstrap only — deploy.sh / wait-drain.sh own the map via kubectl.
  data = {}

  lifecycle {
    ignore_changes = [data]
  }
}

locals {
  api_env_common = {
    HOST          = "0.0.0.0"
    PORT          = "8080"
    COOKIE_SECURE = "true"
    COOKIE_DOMAIN = var.cookie_domain
    CORS_ORIGIN   = var.cors_origin
    RUST_LOG      = var.log_level
  }

  # Same slug rules as iac/scripts/deploy.sh instance_id_from_image.
  server_image_tag = element(split(":", var.server_image), length(split(":", var.server_image)) - 1)
  api_active_instance_id = format(
    "edh-api-%s",
    trimsuffix(trimprefix(replace(lower(local.server_image_tag), "/[^a-z0-9]+/", "-"), "-"), "-")
  )

  # Refreshed from the cluster; ignore_changes keeps scripts' kubectl updates.
  api_peer_images = coalesce(kubernetes_config_map_v1.edh_api_peers.data, {})

  api_instances = merge(
    {
      for id, img in local.api_peer_images : id => { image = img }
      if id != local.api_active_instance_id
    },
    {
      (local.api_active_instance_id) = { image = var.server_image }
    }
  )

  api_active_image = var.server_image
}

check "api_instances_cap" {
  assert {
    condition     = length(local.api_instances) <= var.api_max_instances
    error_message = "api instance count (active + peers) exceeds api_max_instances (${var.api_max_instances})."
  }
}

resource "kubernetes_deployment_v1" "edh_api" {
  for_each = local.api_instances

  wait_for_rollout = true

  metadata {
    name      = each.key
    namespace = local.namespace
    labels = merge(local.common_labels, {
      app                  = each.key
      "mtgfr.io/component" = "api"
      "mtgfr.io/api-role"  = each.key == local.api_active_instance_id ? "active" : "drain"
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

  depends_on = [
    kubernetes_job_v1.edh_migrate,
    kubernetes_config_map_v1.edh_api_peers,
  ]
}

resource "kubernetes_service_v1" "edh_api" {
  for_each = local.api_instances

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
