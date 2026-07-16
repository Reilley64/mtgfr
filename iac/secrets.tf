# Deploy PRD §Variables & secrets / §What mtgfr Terraform owns ("Secrets | DATABASE_URL, tunnel
# token, etc."). Non-workload-specific Secrets live here; workload manifests (api.tf, migrate.tf,
# tunnel.tf) reference them by name via `secret_key_ref` / volume mounts.

resource "kubernetes_secret" "mtgfr_db" {
  metadata {
    name      = "mtgfr-db"
    namespace = local.namespace
  }

  data = {
    DATABASE_URL = local.database_url
  }

  type = "Opaque"
}

resource "kubernetes_secret" "mtgfr_auth" {
  # Reserved — session signing if the server grows one (deploy PRD Settings table). Not consumed
  # by any Deployment yet; wired here so it has a stable home once it is.
  metadata {
    name      = "mtgfr-auth"
    namespace = local.namespace
  }

  data = {
    AUTH_SECRET = var.auth_secret
  }

  type = "Opaque"
}

# Shared secret guarding /admin/drain + /health/drain (deploy PRD §Admin / drain endpoints).
# Consumed as ADMIN_TOKEN by both edh-api and edh-api-drain (api.tf), and by scripts/wait-drain.sh
# via MTGFR_ADMIN_TOKEN so the drain toggle it issues carries the same bearer. May be an empty
# string (var.admin_token default) — the server treats that the same as "no token configured" and
# leaves the routes open, matching local dev.
resource "kubernetes_secret" "mtgfr_admin" {
  metadata {
    name      = "mtgfr-admin"
    namespace = local.namespace
  }

  data = {
    ADMIN_TOKEN = var.admin_token
  }

  type = "Opaque"
}

# Cloudflare Tunnel credentials — the token cloudflared authenticates with. Generated from the
# tunnel resources in tunnel.tf; stored here so all Secret objects have one home.
resource "kubernetes_secret" "cloudflared_token" {
  metadata {
    name      = "cloudflared-token"
    namespace = local.namespace
  }

  data = {
    TUNNEL_TOKEN = data.cloudflare_zero_trust_tunnel_cloudflared_token.edh.token
  }

  type = "Opaque"
}
