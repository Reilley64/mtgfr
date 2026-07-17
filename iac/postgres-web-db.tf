# Create `mtgfr_web` on the shared Postgres instance for SolidStart/Drizzle.
# Official postgres image only bootstraps POSTGRES_DB=mtgfr; this Job is idempotent.
#
# Pod must keep label `app=postgres-create-web-db` (network-policy.tf postgres ingress).

resource "kubernetes_job_v1" "postgres_create_web_db" {
  wait_for_completion = true

  metadata {
    name      = "postgres-create-web-db"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "postgres-create-web-db" })
  }

  timeouts {
    create = "10m"
  }

  spec {
    # Keep failed pods around so `kubectl logs job/…` works after BackoffLimitExceeded.
    ttl_seconds_after_finished = 3600
    backoff_limit              = 3

    template {
      metadata {
        labels = merge(local.common_labels, { app = "postgres-create-web-db" })
      }

      spec {
        restart_policy = "Never"

        container {
          name    = "create-db"
          image   = var.postgres_image
          command = ["/bin/sh", "-ec"]
          args = [<<-EOT
            export PGHOST=${local.postgres_service}
            export PGUSER=mtgfr
            export PGDATABASE=mtgfr
            echo "waiting for postgres…"
            until pg_isready; do sleep 2; done
            echo "checking for mtgfr_web…"
            if psql -v ON_ERROR_STOP=1 -Atc "SELECT 1 FROM pg_database WHERE datname = 'mtgfr_web'" | grep -qx 1; then
              echo "mtgfr_web already exists"
            else
              echo "creating mtgfr_web…"
              psql -v ON_ERROR_STOP=1 -c "CREATE DATABASE mtgfr_web OWNER mtgfr"
            fi
            echo "ok"
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

  depends_on = [
    kubernetes_stateful_set_v1.postgres,
    kubernetes_network_policy_v1.postgres_ingress,
  ]

  # Job spec is mostly immutable; ignore controller-owned selector churn.
  lifecycle {
    ignore_changes = [spec[0].selector]
  }
}
