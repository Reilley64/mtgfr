# Deploy PRD §Postgres — Bitnami Helm. Single primary + PVC; skip CloudNativePG for v1. Backups =
# k3s/PVC snapshots (+ existing etcd/datastore backups) — no separate dump cron until we need one.

resource "helm_release" "postgresql" {
  name       = local.postgres_service
  repository = "https://charts.bitnami.com/bitnami"
  chart      = "postgresql"
  version    = var.postgres_chart_version
  namespace  = local.namespace

  values = [
    yamlencode({
      # fullnameOverride pins the Service name to exactly "postgres" (see locals.tf) — otherwise
      # the Bitnami fullname helper would produce "postgres-postgresql".
      fullnameOverride = local.postgres_service

      auth = {
        username = "mtgfr"
        password = var.mtgfr_db_password
        database = "mtgfr"
      }

      primary = {
        persistence = {
          enabled      = true
          size         = var.postgres_storage_size
          storageClass = var.postgres_storage_class
        }
      }
    })
  ]
}
