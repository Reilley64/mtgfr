# SolidStart BFF: API_UPSTREAM → edh-api (active); WEB_DATABASE_URL → mtgfr_web (Drizzle).

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

          env {
            name  = "WEB_DATABASE_URL"
            value = local.web_database_url
          }

          env {
            name  = "API_UPSTREAM"
            value = "http://edh-api.${local.namespace}.svc:8080"
          }
        }
      }
    }
  }

  depends_on = [
    kubernetes_job_v1.postgres_create_web_db,
    kubernetes_job_v1.edh_web_migrate,
    kubernetes_service_v1.edh_api_active,
  ]
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
