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

#[cfg(test)]
mod tests {
    use super::Deployment;

    #[test]
    fn deserialize_with_status_and_replicas() {
        let json = r#"{"status":{"availableReplicas":3}}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.status.unwrap().available_replicas, Some(3));
    }

    #[test]
    fn deserialize_with_zero_replicas() {
        let json = r#"{"status":{"availableReplicas":0}}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.status.unwrap().available_replicas, Some(0));
    }

    #[test]
    fn deserialize_with_null_available_replicas() {
        let json = r#"{"status":{"availableReplicas":null}}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.status.unwrap().available_replicas, None);
    }

    #[test]
    fn deserialize_with_missing_available_replicas_field() {
        let json = r#"{"status":{}}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.status.unwrap().available_replicas, None);
    }

    #[test]
    fn deserialize_with_missing_status() {
        let json = r#"{}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert!(deployment.status.is_none());
    }

    #[test]
    fn deserialize_with_null_status() {
        let json = r#"{"status":null}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert!(deployment.status.is_none());
    }

    #[test]
    fn deserialize_extra_fields_are_ignored() {
        let json = r#"{"metadata":{"name":"my-deploy"},"status":{"availableReplicas":1,"readyReplicas":1}}"#;
        let deployment: Deployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.status.unwrap().available_replicas, Some(1));
    }
}
