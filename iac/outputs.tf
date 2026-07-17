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

output "grafana_port_forward" {
  description = "kubectl port-forward to open Grafana locally (admin password in secret grafana-admin)."
  value       = "kubectl -n ${local.observability_namespace} port-forward svc/grafana 3000:80"
}

output "grafana_admin_password" {
  description = "Grafana admin password (also in Secret grafana-admin)."
  value       = random_password.grafana_admin.result
  sensitive   = true
}
