use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct K8sConfig {
    pub token_path: PathBuf,
    pub cacrt_path: PathBuf,
    pub service_host: String,
    pub service_port_https: u16,
    pub service_namespace: Option<String>,
    pub service_deployment_name: String,
    pub namespace_file_path: Option<PathBuf>,
}
