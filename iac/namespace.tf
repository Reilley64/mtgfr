# Deploy PRD §Terraform layout / §What mtgfr Terraform owns.

resource "kubernetes_namespace_v1" "terraform" {
  count = var.manage_terraform_namespace ? 1 : 0

  metadata {
    name = var.namespace_terraform
  }
}

resource "kubernetes_namespace_v1" "edh" {
  metadata {
    name = var.namespace_edh
  }
}
