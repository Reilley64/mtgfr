# Sticky proxy for api.edh: cookie `mtgfr-instance` → edh-api / edh-api-drain.
# When api_drain_enabled is false, map drain cookie → edh-api (stale cookies after teardown).
# 404 /admin/* and /health/drain on the public path (NetworkPolicy is L3/L4 only).

resource "kubernetes_config_map" "edh_api_proxy" {
  metadata {
    name      = "edh-api-proxy-nginx"
    namespace = local.namespace
  }

  data = {
    "nginx.conf" = <<-NGINX
      worker_processes auto;
      events {
        worker_connections 1024;
      }

      http {
        resolver kube-dns.kube-system.svc.cluster.local valid=10s;

        # Cookie sticky; drain peer maps to active when api_drain_enabled is false.
        map $cookie_mtgfr_instance $mtgfr_upstream {
          default       edh-api;
          edh-api       edh-api;
          edh-api-drain ${var.api_drain_enabled ? "edh-api-drain" : "edh-api"};
        }

        server {
          listen 8080;

          location ~ ^/(admin/|health/drain) {
            return 404;
          }

          location / {
            proxy_pass http://$mtgfr_upstream.${local.namespace}.svc.cluster.local:8080;
            proxy_http_version 1.1;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header Connection "";

            # SSE (deploy PRD §DNS & Cloudflare — SSE through Cloudflare): never buffer the API
            # hop, and hold connections open for long-lived streams.
            proxy_buffering         off;
            chunked_transfer_encoding off;
            proxy_read_timeout      1h;
            proxy_send_timeout      1h;
          }
        }
      }
    NGINX
  }
}

resource "kubernetes_deployment" "edh_api_proxy" {
  metadata {
    name      = "edh-api-proxy"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api-proxy" })
  }

  spec {
    replicas = 1

    selector {
      match_labels = { app = "edh-api-proxy" }
    }

    template {
      metadata {
        labels = merge(local.common_labels, { app = "edh-api-proxy" })
      }

      spec {
        container {
          name  = "nginx"
          image = "nginx:1.27" # non-distroless by exception — no maintained distroless nginx image; this is infra plumbing, not an application runtime (unlike mtgfr-server/mtgfr-web).

          port {
            container_port = 8080
          }

          volume_mount {
            name       = "nginx-conf"
            mount_path = "/etc/nginx/nginx.conf"
            sub_path   = "nginx.conf"
          }
        }

        volume {
          name = "nginx-conf"

          config_map {
            name = kubernetes_config_map.edh_api_proxy.metadata[0].name
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "edh_api_proxy" {
  metadata {
    name      = "edh-api-proxy"
    namespace = local.namespace
    labels    = merge(local.common_labels, { app = "edh-api-proxy" })
  }

  spec {
    selector = { app = "edh-api-proxy" }

    port {
      port        = 8080
      target_port = 8080
    }
  }
}
