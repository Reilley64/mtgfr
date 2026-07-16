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
        # Single public origin: SolidStart serves the SPA and proxies `/api/*` in-cluster.
        hostname = var.edh_hostname
        service  = "http://${kubernetes_service_v1.edh_web.metadata[0].name}.${local.namespace}.svc:8080"
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

# Preserve the existing ruleset in state when renaming from the old API-hostname rule.
moved {
  from = cloudflare_ruleset.api_edh_no_response_buffering
  to   = cloudflare_ruleset.edh_no_response_buffering
}

# Deploy PRD §DNS & Cloudflare (SSE through Cloudflare, required) — stream responses must not be
# buffered at the edge, or SSE keepalives get held back and players see stalled streams.
# Same-origin BFF: streams go through `edh` (`/api/.../stream/v1`), not a separate API hostname.
resource "cloudflare_ruleset" "edh_no_response_buffering" {
  zone_id     = var.cloudflare_zone_id
  name        = "mtgfr edh — disable response buffering"
  description = "SSE via SolidStart BFF (/api/tables/{table}/stream/v1) must stream unbuffered."
  kind        = "zone"
  phase       = "http_config_settings"

  rules = [
    {
      description = "No response body buffering for edh hostname"
      expression  = "(http.host eq \"${var.edh_hostname}\")"
      action      = "set_config"

      action_parameters = {
        response_body_buffering = "none"
      }
    },
  ]
}

resource "kubernetes_deployment_v1" "cloudflared" {
  wait_for_rollout = true

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
                name = kubernetes_secret_v1.cloudflared_token.metadata[0].name
                key  = "TUNNEL_TOKEN"
              }
            }
          }
        }
      }
    }
  }
}
