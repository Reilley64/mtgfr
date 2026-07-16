# Read by deploy scripts / operators after apply.

output "server_image" {
  description = "Desired/applied active mtgfr-server image (var.server_image)."
  value       = var.server_image
}

output "web_image" {
  description = "The mtgfr-web image applied to the edh-web Deployment."
  value       = var.web_image
}

output "api_active_instance_id" {
  description = "INSTANCE_ID / Deployment name of the API that accepts new tables."
  value       = local.api_active_instance_id
}
