# Headless Service only — Deployments + Service edh-api live in iac/charts/edh (Argo).

locals {
  server_image_tag = element(split(":", var.server_image), length(split(":", var.server_image)) - 1)
  api_active_instance_id = format(
    "edh-api-%s",
    trimsuffix(trimprefix(replace(lower(local.server_image_tag), "/[^a-z0-9]+/", "-"), "-"), "-")
  )

  api_active_image     = var.server_image
  api_headless_service = "edh-api-headless"
}

# Sticky dial for Terminating pods (table_routes → pod DNS).
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
      name        = "http"
      port        = 8080
      target_port = 8080
    }

    # gRPC (ADR 0032): the wire contract's authoritative transport. The BFF dials this port on
    # the pod DNS the headless service resolves for table affinity.
    port {
      name        = "grpc"
      port        = 50051
      target_port = 50051
    }
  }
}
