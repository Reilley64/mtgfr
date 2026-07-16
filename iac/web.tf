# Deploy PRD §Client/server roll order (locked). `web_image` always applies as given — the
# holding pattern that keeps `edh-web` on the previous release while any API drain peer remains
# is owned by the caller (`iac/scripts/deploy.sh`).

resource "kubernetes_deployment_v1" "edh_web" {
  wait_for_rollout = true

  metadata {
    name      = "edh-web"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-web" })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = "edh-web" }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-web" })
      }

      spec {
        container {
          name  = "edh-web"
          image = var.web_image
          # Tags like :1.2.2 are rebuilt in place; IfNotPresent keeps the old digest forever.
          image_pull_policy = "Always"

          port {
            container_port = 8080
          }

          env {
            name  = "HOST"
            value = "0.0.0.0"
          }

          env {
            name  = "PORT"
            value = "8080"
          }

          # SolidStart BFF: cookie sticky → versioned API Services (strip `/api`).
          env {
            name = "API_UPSTREAMS"
            value = jsonencode({
              for id, svc in kubernetes_service_v1.edh_api :
              id => "http://${svc.metadata[0].name}.${local.namespace}.svc:8080"
            })
          }

          env {
            name  = "API_ACTIVE_INSTANCE_ID"
            value = local.api_active_instance_id
          }
        }
      }
    }
  }
}

resource "kubernetes_service_v1" "edh_web" {
  wait_for_load_balancer = false

  metadata {
    name      = "edh-web"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-web" })
  }

  spec {
    selector = { app = "edh-web" }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}
