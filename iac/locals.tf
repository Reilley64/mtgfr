# Shared computed values referenced from multiple files below.

locals {
  namespace = kubernetes_namespace.edh.metadata[0].name

  # Service / StatefulSet name in postgres.tf — must match DATABASE_URL host.
  postgres_service = "postgres"
  # urlencode: the password is user input (terraform.tfvars / TF_VAR_mtgfr_db_password) and can
  # contain URI-reserved characters (@, :, /, ?, #, …) that would otherwise corrupt the DSN or get
  # parsed as part of the host/path instead of the credential.
  database_url = "postgresql://mtgfr:${urlencode(var.mtgfr_db_password)}@${local.postgres_service}:5432/mtgfr"

  common_labels = {
    "app.kubernetes.io/part-of" = "mtgfr"
  }
}
