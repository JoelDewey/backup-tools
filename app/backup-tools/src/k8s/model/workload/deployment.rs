use crate::k8s::model::object_meta::ObjectMeta;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Deployment {
    pub api_version: String,
    pub kind: String,
    pub metadata: ObjectMeta,
    pub spec: DeploymentSpec,
    pub status: Option<DeploymentStatus>,
}

#[derive(Debug, Deserialize)]
pub struct DeploymentSpec {
    pub replicas: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DeploymentStatus {
    pub replicas: i32,
    pub available_replicas: i32,
}
