# Headless Service stays in Terraform (stable selector). Active Service `edh-api` + API/web
# Deployments are Argo-owned (iac/charts/edh) with sync-wave ordering so the Service selector
# flips only after the new API Deployment is healthy; PruneLast drains the prior generation.

locals {
  server_image_tag = element(split(":", var.server_image), length(split(":", var.server_image)) - 1)
  api_active_instance_id = format(
    "edh-api-%s",
    trimsuffix(trimprefix(replace(lower(local.server_image_tag), "/[^a-z0-9]+/", "-"), "-"), "-")
  )

  api_active_image     = var.server_image
  api_headless_service = "edh-api-headless"
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
