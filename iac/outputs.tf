# Read by `iac/scripts/deploy.sh` (via `terraform output -raw ...`) to recover the images that
# were actually applied on the *previous* deploy — plain `-var` inputs don't persist across
# separate `terraform apply` invocations, so the deploy script needs somewhere durable to read
# "what's live right now" from before deciding what the next apply should hold steady vs. bump.
# These simply echo back the input variables, which is correct precisely because both are always
# passed explicitly on every apply (no drift between "what we asked for" and "what's applied").

output "server_image" {
  description = "The mtgfr-server image most recently applied to the edh-api Deployment (not edh-api-drain)."
  value       = var.server_image
}

output "web_image" {
  description = "The mtgfr-web image most recently applied to the edh-web Deployment."
  value       = var.web_image
}
