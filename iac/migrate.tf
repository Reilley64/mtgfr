# Deploy PRD §Database migrations. Job name = short hash of active API image so unchanged
# images are a no-op plan; image bumps force a fresh Job. `wait_for_completion` + Application
# `depends_on` in argocd.tf gate helm param updates on schema. `ttl_seconds_after_finished`
# GC's completed Jobs.

resource "kubernetes_job_v1" "edh_migrate" {
  wait_for_completion = true

  metadata {
    name      = "edh-migrate-${substr(sha256(local.api_active_image), 0, 8)}"
    namespace = local.namespace
    labels    = local.common_labels
  }

  timeouts {
    create = "10m"
  }

  # Job controller owns the selector; importing into v1 otherwise tries to write match_labels = {}.
  lifecycle {
    ignore_changes = [spec[0].selector]
  }

  spec {
    backoff_limit              = 1
    ttl_seconds_after_finished = 300

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-migrate" })
      }

      spec {
        restart_policy = "Never"

        # depends_on the StatefulSet only waits for the object to exist — not for Postgres to
        # accept connections. Without this, the Job often hits connection-refused on first apply.
        init_container {
          name  = "wait-for-postgres"
          image = var.postgres_image
          command = [
            "sh", "-c",
            "until pg_isready -h ${local.postgres_service} -U mtgfr -d mtgfr; do sleep 2; done",
          ]
        }

        container {
          name              = "migrate"
          image             = local.api_active_image
          image_pull_policy = "Always"
          command           = ["/server", "migration", "apply"]

          env {
            name = "DATABASE_URL"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_db.metadata[0].name
                key  = "DATABASE_URL"
              }
            }
          }
        }
      }
    }
  }

  depends_on = [kubernetes_stateful_set_v1.postgres]
}
