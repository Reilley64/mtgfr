# Argo CD install. Workloads remain Terraform-managed in api.tf / web.tf so
# `terraform apply` alone rolls images (SIGTERM drain). When `argocd_repo_url` is set,
# an Application tracks iac/charts/edh for the same values (gitops mirror / future cutover).
# Server TLS stays on (default); reach the UI via kubectl port-forward, not a public Service.

resource "kubernetes_namespace_v1" "argocd" {
  metadata {
    name   = "argocd"
    labels = merge(local.common_labels, { "app.kubernetes.io/name" = "argocd" })
  }
}

resource "helm_release" "argocd" {
  name       = "argocd"
  repository = "https://argoproj.github.io/argo-helm"
  chart      = "argo-cd"
  version    = "7.8.28"
  namespace  = kubernetes_namespace_v1.argocd.metadata[0].name

  depends_on = [kubernetes_namespace_v1.argocd]
}

resource "kubernetes_manifest" "edh_application" {
  count = var.argocd_repo_url == "" ? 0 : 1

  manifest = {
    apiVersion = "argoproj.io/v1alpha1"
    kind       = "Application"
    metadata = {
      name       = "edh"
      namespace  = "argocd"
      finalizers = ["resources-finalizer.argocd.argoproj.io"]
    }
    spec = {
      project = "default"
      source = {
        repoURL        = var.argocd_repo_url
        targetRevision = var.argocd_target_revision
        path           = "iac/charts/edh"
        helm = {
          parameters = [
            { name = "serverImage", value = var.server_image },
            { name = "webImage", value = var.web_image },
            { name = "namespace", value = local.namespace },
            { name = "apiTerminationGraceSeconds", value = tostring(var.api_termination_grace_seconds) },
            { name = "apiActiveInstanceId", value = local.api_active_instance_id },
          ]
        }
      }
      destination = {
        server    = "https://kubernetes.default.svc"
        namespace = local.namespace
      }
      syncPolicy = {
        automated = {
          prune    = false
          selfHeal = false
        }
      }
    }
  }

  depends_on = [helm_release.argocd]
}
