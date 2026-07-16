# Deploy PRD §Terraform layout / §What mtgfr Terraform owns.

resource "kubernetes_namespace" "terraform" {
  count = var.manage_terraform_namespace ? 1 : 0

  metadata {
    name = var.namespace_terraform
  }
}

resource "kubernetes_namespace" "edh" {
  metadata {
    name = var.namespace_edh
  }
}
