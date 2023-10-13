use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct K8sConfig {
    pub kube_token_path: PathBuf,
    pub kube_cacrt_path: PathBuf,
    pub kubernetes_service_host: String,
    pub kubernetes_service_port_https: u16,
    pub service_namespace: String,
    pub service_deployment_name: String,
}
