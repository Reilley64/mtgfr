# Deploy PRD §Cloudflare Tunnel / §DNS & Cloudflare. mtgfr Terraform fully owns the Zero Trust
# tunnel, ingress routes, DNS, and the in-cluster `cloudflared` Deployment + credentials Secret —
# no manual tunnel creation in the Cloudflare UI for steady state.
#
# Resource names below follow the cloudflare/cloudflare v5 provider's Zero Trust naming
# (`cloudflare_zero_trust_tunnel_cloudflared*`). If the installed provider version renames or
# reshapes any of these (the v5 line has moved fast — e.g. the token data source only returned a
# real token from 5.3.0), treat this file as the intended shape and adjust attribute names to
# match `terraform providers schema` for the pinned version in versions.tf.

resource "random_password" "tunnel_secret" {
  length  = 64
  special = false
}

resource "cloudflare_zero_trust_tunnel_cloudflared" "edh" {
  account_id = var.cloudflare_account_id
  name       = var.tunnel_name
  # Remotely-managed config (ingress rules live in Cloudflare, set via the *_config resource
  # below) rather than a local config.yml on the connector.
  config_src    = "cloudflare"
  tunnel_secret = base64encode(random_password.tunnel_secret.result)
}

resource "cloudflare_zero_trust_tunnel_cloudflared_config" "edh" {
  account_id = var.cloudflare_account_id
  tunnel_id  = cloudflare_zero_trust_tunnel_cloudflared.edh.id

  config = {
    ingress = [
      {
        hostname = var.edh_hostname
        service  = "http://${kubernetes_service.edh_web.metadata[0].name}.${local.namespace}.svc:8080"
      },
      {
        hostname = var.api_hostname
        service  = "http://${kubernetes_service.edh_api_proxy.metadata[0].name}.${local.namespace}.svc:8080"
      },
      {
        # Required catch-all — unmatched hostnames get a 404 instead of falling through.
        service = "http_status:404"
      },
    ]
  }
}

data "cloudflare_zero_trust_tunnel_cloudflared_token" "edh" {
  account_id = var.cloudflare_account_id
  tunnel_id  = cloudflare_zero_trust_tunnel_cloudflared.edh.id
}

resource "cloudflare_dns_record" "edh" {
  zone_id = var.cloudflare_zone_id
  name    = "edh"
  type    = "CNAME"
  content = "${cloudflare_zero_trust_tunnel_cloudflared.edh.id}.cfargotunnel.com"
  proxied = true
  ttl     = 1 # "automatic" — required by the API when proxied = true
}

resource "cloudflare_dns_record" "api_edh" {
  zone_id = var.cloudflare_zone_id
  name    = "api.edh"
  type    = "CNAME"
  content = "${cloudflare_zero_trust_tunnel_cloudflared.edh.id}.cfargotunnel.com"
  proxied = true
  ttl     = 1
}

# Deploy PRD §DNS & Cloudflare (SSE through Cloudflare, required) — stream responses on
# api.edh.example.com must not be buffered at the edge, or SSE keepalives get held back and
# players see stalled streams. `response_body_buffering = "none"` is the modern Configuration
# Rules replacement for the legacy Page Rule "Disable Performance" response buffering toggle.
resource "cloudflare_ruleset" "api_edh_no_response_buffering" {
  zone_id     = var.cloudflare_zone_id
  name        = "mtgfr api.edh — disable response buffering"
  description = "SSE (/tables/{table}/stream/v1) must stream through the edge unbuffered."
  kind        = "zone"
  phase       = "http_config_settings"

  rules = [
    {
      description = "No response body buffering for api.edh.example.com"
      expression  = "(http.host eq \"${var.api_hostname}\")"
      action      = "set_config"

      action_parameters = {
        response_body_buffering = "none"
      }
    },
  ]
}

resource "kubernetes_deployment" "cloudflared" {
  metadata {
    name      = "cloudflared"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "cloudflared" })
  }

  spec {
    # HA for the connector, not for capacity — deploy PRD target topology.
    replicas = var.cloudflared_replicas

    selector {
      match_labels = { app = "cloudflared" }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = "cloudflared" })
      }

      spec {
        container {
          name    = "cloudflared"
          image   = var.cloudflared_image
          command = ["cloudflared", "tunnel", "--no-autoupdate", "run"]

          env {
            name = "TUNNEL_TOKEN"
            value_from {
              secret_key_ref {
                name = kubernetes_secret.cloudflared_token.metadata[0].name
                key  = "TUNNEL_TOKEN"
              }
            }
          }
        }
      }
    }
  }
}
