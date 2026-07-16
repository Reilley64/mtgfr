# Deploy PRD §Client/server roll order (locked). `web_image` always applies as given — the
# holding pattern that keeps `edh-web` on the previous release during an API drain window is
# owned by the caller, not by this file: `iac/scripts/deploy.sh` reads the last-applied
# `web_image` (via `output.tf`'s `web_image` output) and passes that same value back explicitly
# on the first (API-roll) apply, then passes the new tag on the second apply once drain empties.
# Never bump both images in one apply — see deploy.sh.

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
          name              = "edh-web"
          image             = var.web_image
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

          # SolidStart BFF proxies same-origin `/api/*` to the sticky nginx (strip `/api`).
          env {
            name  = "API_UPSTREAM"
            value = "http://${kubernetes_service_v1.edh_api_proxy.metadata[0].name}.${local.namespace}.svc:8080"
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
