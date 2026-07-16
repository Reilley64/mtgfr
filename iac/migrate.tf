# Deploy PRD §Database migrations / §Deploy integration. The Job name is derived from a short
# hash of `server_image` rather than `generate_name` — a fixed image tag like `1.2.3` is
# invalid/awkward as a sole DNS-1123 name, but `generate_name` also means Terraform creates (and
# `wait_for_completion` blocks on) a brand-new Job on *every* apply, even when server_image hasn't
# changed — pointless churn on every `terraform apply -var="web_image=..."`-only deploy step.
# Naming on the image hash makes the Job stable (and thus a no-op plan) when the image is
# unchanged, and forces a fresh Job (name changes) whenever it is. `wait_for_completion` blocks
# this resource until the Job finishes, so any resource that `depends_on` it (api.tf) only rolls
# after the schema is current. `ttl_seconds_after_finished` lets the completed Job/Pod get garbage
# collected instead of accumulating one per release forever.

resource "kubernetes_job_v1" "edh_migrate" {
  wait_for_completion = true

  metadata {
    name      = "edh-migrate-${substr(sha256(var.server_image), 0, 8)}"
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
          image             = var.server_image
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
