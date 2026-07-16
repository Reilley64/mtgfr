# Deploy PRD §Terraform (state — kubernetes backend). State is a Secret, locks are Leases, both
# stored on the remote k3s cluster — the apply machine never keeps prod state on local disk after
# `init`; every plan/apply reads/writes state through the k3s API (same kubeconfig the providers
# use, in providers.tf).
#
# Bootstrap (one-time, from the apply machine, before the first `terraform init`):
#
#   kubectl --kubeconfig "$KUBE_CONFIG_PATH" create namespace terraform
#
# The kubeconfig user needs `secrets` + `coordination.k8s.io/leases` access in that namespace.
#
# `config_path` is intentionally omitted from this block — backend blocks cannot interpolate
# `var.*`, so the kubeconfig path is supplied at init time instead of baked into committed config:
#
#   terraform init -backend-config="config_path=$KUBE_CONFIG_PATH"
#
# or simply export KUBE_CONFIG_PATH before `terraform init` (the kubernetes backend and the
# kubernetes/helm providers both honor it as a fallback to their respective `config_path`).
terraform {
  backend "kubernetes" {
    secret_suffix = "mtgfr"
    namespace     = "terraform"
  }
}
