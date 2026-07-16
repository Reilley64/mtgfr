# Read by `iac/scripts/deploy.sh` (via `terraform output`) to recover live instances / images.

output "server_image" {
  description = "Image of the active API instance (api_active_instance_id)."
  value       = local.api_active_image
}

output "web_image" {
  description = "The mtgfr-web image most recently applied to the edh-web Deployment."
  value       = var.web_image
}

output "api_active_instance_id" {
  description = "INSTANCE_ID of the API that accepts new tables."
  value       = var.api_active_instance_id
}

output "api_instances" {
  description = "Map of INSTANCE_ID → image for all live API Deployments (active + draining)."
  value       = { for id, inst in var.api_instances : id => inst.image }
}

output "api_drain_instance_ids" {
  description = "INSTANCE_IDs that are not active (candidates for drain GC)."
  value       = [for id in keys(var.api_instances) : id if id != var.api_active_instance_id]
}
