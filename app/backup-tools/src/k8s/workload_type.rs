use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkloadType {
    Deployment,
    StatefulSet,
}

#[cfg(test)]
mod tests {
    use super::WorkloadType;
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    fn deserialize(s: &str) -> Result<WorkloadType, serde::de::value::Error> {
        WorkloadType::deserialize(s.into_deserializer())
    }

    #[test]
    fn deserialize_deployment() {
        let result = deserialize("DEPLOYMENT");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), WorkloadType::Deployment));
    }

    #[test]
    fn deserialize_statefulset() {
        let result = deserialize("STATEFULSET");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), WorkloadType::StatefulSet));
    }

    #[test]
    fn deserialize_unknown_variant_returns_error() {
        assert!(deserialize("deployment").is_err());
        assert!(deserialize("StatefulSet").is_err());
        assert!(deserialize("").is_err());
    }
}