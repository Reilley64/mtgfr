# SolidStart BFF Service. Deployment is Argo-owned (iac/charts/edh).

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
