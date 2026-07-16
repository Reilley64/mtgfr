# Read by deploy / wait-drain scripts. Peer map mirrors ConfigMap edh-api-peers.

output "server_image" {
  description = "Desired/applied active mtgfr-server image (var.server_image)."
  value       = var.server_image
}

output "web_image" {
  description = "The mtgfr-web image most recently applied to the edh-web Deployment."
  value       = var.web_image
}

output "api_active_instance_id" {
  description = "INSTANCE_ID of the API that accepts new tables (derived from server_image tag)."
  value       = local.api_active_instance_id
}

output "api_instances" {
  description = "Map of INSTANCE_ID → image for all live API Deployments (active + draining)."
  value       = { for id, inst in local.api_instances : id => inst.image }
}

output "api_peer_images" {
  description = "Drain peer INSTANCE_ID → image (ConfigMap edh-api-peers data)."
  value = {
    for id, img in local.api_instances : id => img
    if id != local.api_active_instance_id
  }
}

output "api_drain_instance_ids" {
  description = "INSTANCE_IDs that are not active (candidates for drain GC)."
  value       = [for id in keys(local.api_instances) : id if id != local.api_active_instance_id]
}
