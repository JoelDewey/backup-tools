use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
    pub status: Option<DeploymentStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStatus {
    pub available_replicas: Option<i32>,
}
