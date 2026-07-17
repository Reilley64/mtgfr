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
  description = "Namespace holding all edh workloads (web, versioned API instances, postgres, cloudflared)."
  type        = string
  default     = "edh"
}

variable "namespace_observability" {
  description = "Namespace for self-hosted LGTM (Grafana/Loki/Tempo/Prometheus) + Alloy."
  type        = string
  default     = "observability"
}

variable "observability_storage_size" {
  description = "PVC size for Loki, Tempo, and Prometheus (local-path on k3s)."
  type        = string
  default     = "10Gi"
}

variable "otel_exporter_otlp_endpoint" {
  description = "OTLP HTTP endpoint for edh-api and edh-web. Empty disables export in the chart (local/dev)."
  type        = string
  default     = "http://alloy.observability.svc:4318"
}

variable "faro_collect_upstream" {
  description = "Alloy Faro collect URL the BFF proxies /api/faro/collect to."
  type        = string
  default     = "http://alloy.observability.svc:12347/collect"
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
  description = "Cloudflare zone ID for example.com (DNS record for edh lives here)."
  type        = string
}

variable "dns_zone" {
  description = "Root DNS zone name (deploy PRD §DNS & Cloudflare)."
  type        = string
  default     = "example.com"
}

variable "edh_hostname" {
  description = "Single public hostname for the SolidStart BFF (SPA + `/api` proxy)."
  type        = string
  default     = "edh.example.com"
}

variable "argocd_repo_url" {
  description = "Git repo URL for the Argo Application (iac/charts/edh). Required — Argo owns API/web Deployments."
  type        = string
}

variable "argocd_target_revision" {
  description = "Git revision for the Argo Application."
  type        = string
  default     = "HEAD"
}

variable "tunnel_name" {
  description = "Display name for the Cloudflare Zero Trust tunnel."
  type        = string
  default     = "mtgfr-edh"
}

variable "cloudflared_replicas" {
  description = "cloudflared connector replicas. Default 1 (friend-group / small cluster); set 2 for connector HA."
  type        = number
  default     = 1
}

# ── Images / API instances ───────────────────────────────────────────────────────────────────────
# Public GHCR packages (deploy PRD — no imagePullSecrets). Never a moving `latest` tag; pin
# explicit release versions. Operator sets only `server_image` (desired active) + `web_image`.
# Rolls: terraform apply updates Deployments; Terminating API pods drain in-process (ADR 0030).

variable "server_image" {
  description = "Desired active mtgfr-server image. INSTANCE_ID is derived as edh-api-<slug(tag)>."
  type        = string
}

variable "api_termination_grace_seconds" {
  description = "Max game length ceiling: SIGTERM drain wait before kube SIGKILL (default 24h)."
  type        = number
  default     = 86400
}

variable "web_image" {
  description = "mtgfr-web (SolidStart BFF) image. May roll with server_image (expand-only wire across Terminating API pods)."
  type        = string
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
  description = "Domain attribute for the auth session cookie. Empty = host-only on edh (same-origin BFF)."
  type        = string
  default     = ""
}

variable "cors_origin" {
  description = "Allowed CORS origin for the API. Empty when the browser is same-origin via the SolidStart BFF (no browser CORS)."
  type        = string
  default     = ""
}

variable "auth_secret" {
  description = "Reserved — session signing if added later. Not yet consumed by the server; wired through secrets.tf so it has a home once it is."
  type        = string
  sensitive   = true
  default     = ""
}

variable "log_level" {
  description = "RUST_LOG value for all API instances."
  type        = string
  default     = "info"
}

# ── Cloudflared connector ───────────────────────────────────────────────────────────────────────

variable "cloudflared_image" {
  description = "cloudflared connector image. Pinned to an explicit release — never `:latest` (moving tags make rollouts non-reproducible and unreviewable)."
  type        = string
  default     = "cloudflare/cloudflared:2026.6.0"
}
