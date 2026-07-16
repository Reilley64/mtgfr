# Deploy PRD §Terraform (providers). Remote k3s only — never assumes Terraform runs on the
# cluster node (no `in_cluster_config`). No Docker-over-SSH provider, no Traefik labels, no
# homelab data sources.

provider "kubernetes" {
  config_path = var.kubeconfig_path
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}
