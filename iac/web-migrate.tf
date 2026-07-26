# Drizzle migrations for mtgfr_web (Foldkit/Nitro lobby / table_routes).
# Mirrors iac/migrate.tf: wait_for_completion so edh-web only rolls after schema is current.
# Uses oven/bun + drizzle-kit (web image is distroless Bun and has no migrate entrypoint).
# Pins must match client/package.json (drizzle-orm / drizzle-kit).

locals {
  web_mig_root  = "${path.module}/../client/db/migrations"
  web_mig_files = fileset(local.web_mig_root, "**")
  # Bump when the Job script body changes so a new Job name re-runs migrate
  # (Completed Jobs are not re-executed on in-place spec edits alone).
  web_mig_script_rev = "drizzle-v3-reconcile-1"
  web_migrations_hash = substr(sha256(join("", concat([
    for f in sort(tolist(local.web_mig_files)) : filesha256("${local.web_mig_root}/${f}")
  ], [local.web_mig_script_rev]))), 0, 8)
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
    backoff_limit              = 2
    ttl_seconds_after_finished = 3600

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
          image             = "oven/bun:1.3.14"
          image_pull_policy = "IfNotPresent"
          working_dir       = "/work"
          command           = ["/bin/sh", "-ec"]
          # drizzle-kit migrate needs a Postgres driver (`pg`) at runtime — kit alone is not enough.
          # Pre-squash journals must be reconciled to the v3 baseline before kit will migrate
          # (see production-topology-and-operations.md §Database migrations).
          args = [<<-EOT
            set -e
            mkdir -p /work/db
            cp -a /migrations/. /work/db/migrations/
            cat > /work/package.json <<'PKG'
            {"name":"edh-web-migrate","private":true,"type":"module","dependencies":{"drizzle-orm":"1.0.0-rc.4","pg":"8.16.3"},"devDependencies":{"drizzle-kit":"1.0.0-rc.4"}}
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
            cat > /work/reconcile-v3-baseline.mjs <<'JS'
            import { createHash } from "node:crypto";
            import { readdirSync, readFileSync } from "node:fs";
            import { join } from "node:path";
            import pg from "pg";

            const url = process.env.WEB_DATABASE_URL;
            if (!url) throw new Error("WEB_DATABASE_URL required");
            const root = "/work/db/migrations";
            const folders = readdirSync(root, { withFileTypes: true })
              .filter((d) => d.isDirectory())
              .map((d) => d.name)
              .sort();
            if (folders.length === 0) throw new Error("no migration folders found");

            const client = new pg.Client({ connectionString: url });
            await client.connect();

            const tables = await client.query(
              `SELECT 1 FROM information_schema.tables
               WHERE table_schema = 'public' AND table_name = 'lobby_seats'`,
            );
            if (tables.rows.length === 0) {
              console.log("reconcile: no lobby_seats yet — leave journal for drizzle-kit migrate");
              await client.end();
              process.exit(0);
            }

            await client.query(
              `ALTER TABLE "lobby_seats" ADD COLUMN IF NOT EXISTS "gravatar_hash" text DEFAULT '' NOT NULL`,
            );
            const { rows: gravatar } = await client.query(
              `SELECT 1 AS ok FROM information_schema.columns
               WHERE table_schema = 'public' AND table_name = 'lobby_seats'
                 AND column_name = 'gravatar_hash'`,
            );
            if (gravatar.length !== 1) {
              throw new Error("schema assert failed: lobby_seats.gravatar_hash missing");
            }

            const journalExists = await client.query(
              `SELECT 1 FROM information_schema.tables
               WHERE table_schema = 'drizzle' AND table_name = '__drizzle_migrations'`,
            );
            if (journalExists.rows.length === 0) {
              console.log("reconcile: no journal table — leave for drizzle-kit migrate");
              await client.end();
              process.exit(0);
            }

            const { rows: existing } = await client.query(
              `SELECT id, hash, created_at FROM drizzle.__drizzle_migrations ORDER BY id`,
            );

            const baselineName = folders[0];
            const sql = readFileSync(join(root, baselineName, "migration.sql"));
            const baselineHash = createHash("sha256").update(sql).digest("hex");
            const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})_/.exec(baselineName);
            if (!m) throw new Error(`unexpected migration folder name: $${baselineName}`);
            const createdAt = Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]);

            if (existing.length === 1 && existing[0].hash === baselineHash) {
              console.log(`reconcile: already on v3 baseline $${baselineName}`);
              await client.end();
              process.exit(0);
            }

            if (existing.length === 0) {
              console.log("reconcile: empty journal with existing tables — seed baseline");
            } else {
              console.log(
                `reconcile: replacing $${existing.length} pre-squash journal row(s) with $${baselineName}`,
              );
            }

            await client.query("BEGIN");
            await client.query("DELETE FROM drizzle.__drizzle_migrations");
            const cols = await client.query(
              `SELECT column_name FROM information_schema.columns
               WHERE table_schema = 'drizzle' AND table_name = '__drizzle_migrations'`,
            );
            const names = new Set(cols.rows.map((r) => r.column_name));
            if (names.has("name")) {
              await client.query(
                `INSERT INTO drizzle.__drizzle_migrations (hash, created_at, name)
                 VALUES ($1, $2, $3)`,
                [baselineHash, createdAt, baselineName],
              );
            } else {
              await client.query(
                `INSERT INTO drizzle.__drizzle_migrations (hash, created_at)
                 VALUES ($1, $2)`,
                [baselineHash, createdAt],
              );
            }
            await client.query(
              `SELECT setval(
                 pg_get_serial_sequence('drizzle.__drizzle_migrations', 'id'),
                 (SELECT COALESCE(MAX(id), 1) FROM drizzle.__drizzle_migrations)
               )`,
            );
            await client.query("COMMIT");
            console.log(`reconcile: journal now $${baselineName} hash=$${baselineHash}`);
            await client.end();
            JS
            bun /work/reconcile-v3-baseline.mjs
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
    kubernetes_network_policy_v1.postgres_ingress,
  ]
}
