# Deploy PRD §Variables & secrets. Populate via `terraform.tfvars` (gitignored — copy
# `terraform.tfvars.example`) or environment variables (`TF_VAR_*`). No secrets have defaults.

# ── Apply machine / cluster access ──────────────────────────────────────────────────────────────

variable "kubeconfig_path" {
  description = "Path on the apply machine to a kubeconfig that reaches the remote k3s API. Also honored via KUBE_CONFIG_PATH by the kubernetes backend and providers."
  type        = string
}

variable "manage_terraform_namespace" {
  description = "Whether Terraform should create the `terraform` namespace (state Secret/Lease). Leave false if it was bootstrapped by hand per the backend.tf bootstrap step, to avoid fighting an out-of-band-created namespace on first apply."
  type        = bool
  default     = false
}

variable "namespace_terraform" {
  description = "Namespace holding the Terraform state Secret + lock Lease."
  type        = string
  default     = "terraform"
}

variable "namespace_edh" {
  description = "Namespace holding all edh workloads (web, api, api-drain, api-proxy, postgres, cloudflared)."
  type        = string
  default     = "edh"
}

# ── Cloudflare ───────────────────────────────────────────────────────────────────────────────────

variable "cloudflare_api_token" {
  description = "Cloudflare API token with DNS edit + Zero Trust tunnel edit scope on the example.com zone/account."
  type        = string
  sensitive   = true
}

variable "cloudflare_account_id" {
  description = "Cloudflare account ID that owns the Zero Trust tunnel."
  type        = string
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for example.com (DNS records for edh + api.edh live here)."
  type        = string
}

variable "dns_zone" {
  description = "Root DNS zone name (deploy PRD §DNS & Cloudflare)."
  type        = string
  default     = "example.com"
}

variable "edh_hostname" {
  description = "Public hostname for the static client (deploy PRD §Target topology)."
  type        = string
  default     = "edh.example.com"
}

variable "api_hostname" {
  description = "Public hostname for the API + SSE, proxied through edh-api-proxy."
  type        = string
  default     = "api.edh.example.com"
}

variable "tunnel_name" {
  description = "Display name for the Cloudflare Zero Trust tunnel."
  type        = string
  default     = "mtgfr-edh"
}

variable "cloudflared_replicas" {
  description = "Number of cloudflared connector replicas (deploy PRD: 2, for connector HA)."
  type        = number
  default     = 2
}

# ── Images ───────────────────────────────────────────────────────────────────────────────────────
# Public GHCR packages (deploy PRD — no imagePullSecrets). Never a moving `latest` tag; pin
# explicit release versions here.

variable "server_image" {
  description = "mtgfr-server image for the active edh-api Deployment (and the migrate Job)."
  type        = string
}

variable "server_image_drain" {
  description = "mtgfr-server image for the draining edh-api-drain Deployment. Only used while api_drain_enabled = true; typically the previous server_image value during a roll."
  type        = string
  default     = ""
}

variable "web_image" {
  description = "mtgfr-web (`mtgfr static`) image for the edh-web Deployment. Deploy PRD — bump this only after the drain window closes; `iac/scripts/deploy.sh` holds this at the previously-applied tag during the API roll and only passes the new tag once wait-drain.sh confirms active_tables=0."
  type        = string
}

# ── Rolling deploy ───────────────────────────────────────────────────────────────────────────────

variable "api_drain_enabled" {
  description = "Whether the edh-api-drain Deployment/Service exist. True only during a roll (old instance kept alive on the previous image while edh-api serves the new one); false in steady state once drain empties and the peer is torn down."
  type        = bool
  default     = false
}

# ── Database ─────────────────────────────────────────────────────────────────────────────────────

variable "mtgfr_db_password" {
  description = "Password for the `mtgfr` Postgres role. Composed into DATABASE_URL in Terraform (secrets.tf)."
  type        = string
  sensitive   = true
}

variable "postgres_image" {
  description = "Official Postgres container image (pin a major tag; never floating latest)."
  type        = string
  default     = "postgres:17"
}

variable "postgres_storage_size" {
  description = "PVC size for the Postgres primary."
  type        = string
  default     = "8Gi"
}

variable "postgres_storage_class" {
  description = "StorageClass for the Postgres PVC. Empty string uses the cluster default (e.g. k3s local-path)."
  type        = string
  default     = ""
}

# ── Server runtime (Settings — deploy PRD §Configuration) ──────────────────────────────────────

variable "cookie_domain" {
  description = "Domain attribute for the auth session cookie. NOT used for the mtgfr-instance affinity cookie, which stays host-only on the API regardless of this value."
  type        = string
  default     = ".example.com"
}

variable "cors_origin" {
  description = "Allowed CORS origin for the API."
  type        = string
  default     = "https://edh.example.com"
}

variable "auth_secret" {
  description = "Reserved — session signing if added later. Not yet consumed by the server; wired through secrets.tf so it has a home once it is."
  type        = string
  sensitive   = true
  default     = ""
}

variable "admin_token" {
  description = "Bearer token guarding POST /admin/drain and GET /health/drain (deploy PRD §Admin / drain endpoints). Defense in depth on top of the NetworkPolicy that already keeps these routes cluster-internal; matches the server's `admin_token` Settings default so an unset token behaves the same on both sides. Empty leaves them unauthenticated. Set a strong value and pass it back via MTGFR_ADMIN_TOKEN to scripts/wait-drain.sh."
  type        = string
  sensitive   = true
  default     = ""
}

variable "log_level" {
  description = "RUST_LOG value for edh-api / edh-api-drain."
  type        = string
  default     = "info"
}

# ── Cloudflared connector ───────────────────────────────────────────────────────────────────────

variable "cloudflared_image" {
  description = "cloudflared connector image. Pinned to an explicit release — never `:latest` (moving tags make rollouts non-reproducible and unreviewable)."
  type        = string
  default     = "cloudflare/cloudflared:2026.6.0"
}
