# Argo CD owns API + web Deployments and Service edh-api (iac/charts/edh). Terraform apply
# updates Application helm params (images / grace / active instance id); it does not wait on
# pod termination grace. Sync waves: Deployment (0) then edh-api Service (1). PruneLast deletes
# the prior per-tag API Deployment after the new generation is up → SIGTERM drain.

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
            { name = "apiHeadlessService", value = local.api_headless_service },
            { name = "cookieDomain", value = var.cookie_domain },
            { name = "corsOrigin", value = var.cors_origin },
            { name = "logLevel", value = var.log_level },
            { name = "dbSecretName", value = kubernetes_secret_v1.mtgfr_db.metadata[0].name },
            { name = "adminSecretName", value = kubernetes_secret_v1.mtgfr_admin.metadata[0].name },
          ]
        }
      }
      destination = {
        server    = "https://kubernetes.default.svc"
        namespace = local.namespace
      }
      syncPolicy = {
        automated = {
          prune    = true
          selfHeal = true
        }
        syncOptions = [
          "PruneLast=true",
        ]
      }
    }
  }

  # Image bumps recreate migrate Jobs (name = image hash); wait_for_completion finishes before
  # this Application CR is updated, so Argo never syncs a new server image ahead of schema.
  depends_on = [
    helm_release.argocd,
    kubernetes_job_v1.edh_migrate,
    kubernetes_job_v1.postgres_create_web_db,
    kubernetes_job_v1.edh_web_migrate,
    kubernetes_secret_v1.mtgfr_db,
    kubernetes_secret_v1.mtgfr_admin,
  ]
}
