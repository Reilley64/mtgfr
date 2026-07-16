# Deploy PRD §Terraform layout. Pinned provider versions — the apply machine is a workstation,
# not the k3s host (see providers.tf); keep this file in sync with `terraform init` output.

terraform {
  required_version = ">= 1.9"

  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 3.2"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.22"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.9"
    }
  }
}
