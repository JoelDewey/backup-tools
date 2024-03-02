use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkloadType {
    Deployment,
    StatefulSet,
}