# Card image CDN — OpenSpec: openspec/specs/production-and-ops/spec.md (Card art CDN)
#
# A Worker in front of an R2 bucket, filled on miss from Scryfall. Nothing metered sits in the
# request path: Workers' free 100k/day rejects rather than bills, R2 egress is free, and no
# Cloudflare Images subscription exists to bill against. Only R2 storage accrues, and it is
# bounded by the card catalog rather than by traffic (~20 GB fully warm, ~$0.15/month).
# This ceiling assumes the Workers Free plan — on Workers Paid, the observability block below
# puts metered Workers Logs in the request path.

locals {
  card_cdn_hostname = "edh-images.${var.dns_zone}"
}

resource "cloudflare_r2_bucket" "card_images" {
  account_id = var.cloudflare_account_id
  name       = "edh-card-images"
  location   = "wnam"
}

resource "cloudflare_workers_script" "card_cdn" {
  account_id  = var.cloudflare_account_id
  script_name = "edh-card-cdn"

  # Uploaded as a single module — no wrangler, no bundler. `content_sha256` is what makes
  # Terraform notice edits to the .js file.
  content_file   = "${path.module}/workers/card-cdn.js"
  content_sha256 = filesha256("${path.module}/workers/card-cdn.js")
  main_module    = "card-cdn.js"

  compatibility_date = "2026-07-30"

  bindings = [
    {
      # The Worker reads `env.CARDS` — keep this name in step with iac/workers/card-cdn.js.
      name        = "CARDS"
      type        = "r2_bucket"
      bucket_name = cloudflare_r2_bucket.card_images.name
    },
  ]

  observability = {
    enabled = true
  }
}

# First-level subdomain, so free Universal SSL covers the certificate. This resource also
# creates the DNS record — no separate cloudflare_dns_record.
resource "cloudflare_workers_custom_domain" "card_cdn" {
  account_id = var.cloudflare_account_id
  zone_id    = var.cloudflare_zone_id
  hostname   = local.card_cdn_hostname
  service    = cloudflare_workers_script.card_cdn.script_name
}
