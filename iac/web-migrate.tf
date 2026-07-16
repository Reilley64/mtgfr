# Drizzle migrations for mtgfr_web (SolidStart lobby / table_routes).
# Mirrors iac/migrate.tf: wait_for_completion so edh-web only rolls after schema is current.
# Uses oven/bun + drizzle-kit (web image is distroless Node and has no migrate entrypoint).

locals {
  web_mig_root  = "${path.module}/../client/db/migrations"
  web_mig_files = fileset(local.web_mig_root, "**")
  web_migrations_hash = substr(sha256(join("", [
    for f in sort(tolist(local.web_mig_files)) : filesha256("${local.web_mig_root}/${f}")
  ])), 0, 8)
}

resource "kubernetes_config_map_v1" "edh_web_migrations" {
  metadata {
    name      = "edh-web-migrations-${local.web_migrations_hash}"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-web-migrate" })
  }

  # Keys flatten nested paths (meta/_journal.json → meta__journal.json); Job unflattens.
  data = {
    for f in local.web_mig_files :
    replace(f, "/", "__") => file("${local.web_mig_root}/${f}")
  }
}

resource "kubernetes_job_v1" "edh_web_migrate" {
  wait_for_completion = true

  metadata {
    name      = "edh-web-migrate-${local.web_migrations_hash}"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-web-migrate" })
  }

  timeouts {
    create = "10m"
  }

  lifecycle {
    ignore_changes = [spec[0].selector]
  }

  spec {
    backoff_limit              = 1
    ttl_seconds_after_finished = 300

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-web-migrate" })
      }

      spec {
        restart_policy = "Never"

        init_container {
          name  = "wait-for-postgres"
          image = var.postgres_image
          command = [
            "sh", "-c",
            "until pg_isready -h ${local.postgres_service} -U mtgfr -d mtgfr_web; do sleep 2; done",
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

        init_container {
          name    = "unflatten-migrations"
          image   = var.postgres_image
          command = ["/bin/sh", "-ec"]
          args = [<<-EOT
            mkdir -p /out
            for f in /raw/*; do
              [ -f "$f" ] || continue
              rel=$(basename "$f" | sed 's|__|/|g')
              mkdir -p "/out/$(dirname "$rel")"
              cp "$f" "/out/$rel"
            done
          EOT
          ]
          volume_mount {
            name       = "raw"
            mount_path = "/raw"
            read_only  = true
          }
          volume_mount {
            name       = "migrations"
            mount_path = "/out"
          }
        }

        container {
          name              = "migrate"
          image             = "oven/bun:1"
          image_pull_policy = "IfNotPresent"
          working_dir       = "/work"
          command           = ["/bin/sh", "-ec"]
          args = [<<-EOT
            set -e
            mkdir -p /work/db
            cp -a /migrations/. /work/db/migrations/
            cat > /work/package.json <<'PKG'
            {
              "name": "edh-web-migrate",
              "private": true,
              "type": "module",
              "devDependencies": {
                "drizzle-kit": "0.31.10",
                "drizzle-orm": "0.45.2"
              }
            }
            PKG
            cat > /work/drizzle.config.ts <<'CFG'
            import { defineConfig } from "drizzle-kit";
            export default defineConfig({
              out: "./db/migrations",
              dialect: "postgresql",
              dbCredentials: { url: process.env.WEB_DATABASE_URL },
            });
            CFG
            bun install --no-save
            bunx drizzle-kit migrate
          EOT
          ]

          env {
            name = "WEB_DATABASE_URL"
            value_from {
              secret_key_ref {
                name = kubernetes_secret_v1.mtgfr_db.metadata[0].name
                key  = "WEB_DATABASE_URL"
              }
            }
          }

          volume_mount {
            name       = "migrations"
            mount_path = "/migrations"
            read_only  = true
          }
        }

        volume {
          name = "raw"
          config_map {
            name = kubernetes_config_map_v1.edh_web_migrations.metadata[0].name
          }
        }

        volume {
          name = "migrations"
          empty_dir {}
        }
      }
    }
  }

  depends_on = [
    kubernetes_job_v1.postgres_create_web_db,
    kubernetes_config_map_v1.edh_web_migrations,
    kubernetes_secret_v1.mtgfr_db,
  ]
}
