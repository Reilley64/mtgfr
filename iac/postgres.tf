# Deploy PRD §Postgres — official `postgres` image (StatefulSet + Service). Single primary + PVC;
# skip CloudNativePG / Bitnami for v1. Backups = k3s/PVC snapshots (+ existing etcd/datastore
# backups) — no separate dump cron until we need one.
#
# Service name is `postgres` so DATABASE_URL (`locals.tf`) stays `…@postgres:5432/mtgfr`.

resource "kubernetes_secret" "postgres" {
  metadata {
    name      = local.postgres_service
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = local.postgres_service })
  }

  data = {
    POSTGRES_PASSWORD = var.mtgfr_db_password
  }

  type = "Opaque"
}

resource "kubernetes_service" "postgres" {
  metadata {
    name      = local.postgres_service
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = local.postgres_service })
  }

  spec {
    selector = { app = local.postgres_service }

    port {
      name        = "postgresql"
      port        = 5432
      target_port = 5432
    }
  }
}

resource "kubernetes_stateful_set" "postgres" {
  wait_for_rollout = true

  metadata {
    name      = local.postgres_service
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = local.postgres_service })
  }

  spec {
    service_name = local.postgres_service
    replicas     = 1

    selector {
      match_labels = { app = local.postgres_service }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = local.postgres_service })
      }

      spec {
        container {
          name  = "postgresql"
          image = var.postgres_image

          port {
            name           = "postgresql"
            container_port = 5432
          }

          env {
            name  = "POSTGRES_USER"
            value = "mtgfr"
          }

          env {
            name  = "POSTGRES_DB"
            value = "mtgfr"
          }

          env {
            name = "POSTGRES_PASSWORD"
            value_from {
              secret_key_ref {
                name = kubernetes_secret.postgres.metadata[0].name
                key  = "POSTGRES_PASSWORD"
              }
            }
          }

          # Avoid mounting the volume root (lost+found) as PGDATA.
          env {
            name  = "PGDATA"
            value = "/var/lib/postgresql/data/pgdata"
          }

          volume_mount {
            name       = "data"
            mount_path = "/var/lib/postgresql/data"
          }

          readiness_probe {
            exec {
              command = ["pg_isready", "-U", "mtgfr", "-d", "mtgfr"]
            }
            initial_delay_seconds = 5
            period_seconds        = 10
          }

          liveness_probe {
            exec {
              command = ["pg_isready", "-U", "mtgfr", "-d", "mtgfr"]
            }
            initial_delay_seconds = 30
            period_seconds        = 10
          }
        }
      }
    }

    volume_claim_template {
      metadata {
        name = "data"
      }

      spec {
        access_modes = ["ReadWriteOnce"]

        resources {
          requests = {
            storage = var.postgres_storage_size
          }
        }

        storage_class_name = var.postgres_storage_class == "" ? null : var.postgres_storage_class
      }
    }
  }

  depends_on = [kubernetes_secret.postgres]
}
