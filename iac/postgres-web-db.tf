# Create `mtgfr_web` on the shared Postgres instance for SolidStart/Drizzle.
# Official postgres image only bootstraps POSTGRES_DB=mtgfr; this Job is idempotent.

resource "kubernetes_job_v1" "postgres_create_web_db" {
  wait_for_completion = true

  metadata {
    name      = "postgres-create-web-db"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "postgres-create-web-db" })
  }

  spec {
    ttl_seconds_after_finished = 300
    backoff_limit              = 6

    template {
      metadata {
        labels = merge(local.common_labels, { app = "postgres-create-web-db" })
      }

      spec {
        restart_policy = "OnFailure"

        container {
          name    = "create-db"
          image   = var.postgres_image
          command = ["/bin/sh", "-ec"]
          args = [<<-EOT
            until pg_isready -h ${local.postgres_service} -U mtgfr -d mtgfr; do sleep 2; done
            exists=$(psql -h ${local.postgres_service} -U mtgfr -d mtgfr -Atc "SELECT 1 FROM pg_database WHERE datname = 'mtgfr_web'")
            if [ "$exists" != "1" ]; then
              psql -h ${local.postgres_service} -U mtgfr -d mtgfr -v ON_ERROR_STOP=1 -c "CREATE DATABASE mtgfr_web OWNER mtgfr"
            fi
          EOT
          ]

          env {
            name = "PGPASSWORD"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.postgres.metadata[0].name
                key  = "POSTGRES_PASSWORD"
              }
            }
          }
        }
      }
    }
  }

  depends_on = [kubernetes_stateful_set_v1.postgres]

  # Job object is immutable after completion; keep first successful create.
  lifecycle {
    ignore_changes = all
  }
}
