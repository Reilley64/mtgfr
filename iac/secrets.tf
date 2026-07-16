# Deploy PRD §Variables & secrets / §What mtgfr Terraform owns ("Secrets | DATABASE_URL, tunnel
# token, etc."). Non-workload-specific Secrets live here; workload manifests (api.tf, migrate.tf,
# tunnel.tf) reference them by name via `secret_key_ref` / volume mounts.

resource "kubernetes_secret_v1" "mtgfr_db" {
  wait_for_service_account_token = false

  metadata {
    name      = "mtgfr-db"
    namespace = local.namespace
  }

  data = {
    DATABASE_URL     = local.database_url
    WEB_DATABASE_URL = local.web_database_url
  }

  type = "Opaque"
}

resource "kubernetes_secret_v1" "mtgfr_auth" {
  wait_for_service_account_token = false

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

# Shared secret for GET /health/drain (`ADMIN_TOKEN`). Empty = unauthenticated (matches server default).
resource "kubernetes_secret_v1" "mtgfr_admin" {
  wait_for_service_account_token = false

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
resource "kubernetes_secret_v1" "cloudflared_token" {
  wait_for_service_account_token = false

  metadata {
    name      = "cloudflared-token"
    namespace = local.namespace
  }

  data = {
    TUNNEL_TOKEN = data.cloudflare_zero_trust_tunnel_cloudflared_token.edh.token
  }

  type = "Opaque"
}
