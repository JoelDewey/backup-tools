use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupType {
    Incremental,
    Compressed,
}

#[cfg(test)]
mod tests {
    use super::BackupType;
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    fn deserialize(s: &str) -> Result<BackupType, serde::de::value::Error> {
        BackupType::deserialize(s.into_deserializer())
    }

    #[test]
    fn deserialize_incremental() {
        let result = deserialize("INCREMENTAL");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), BackupType::Incremental));
    }

    #[test]
    fn deserialize_compressed() {
        let result = deserialize("COMPRESSED");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), BackupType::Compressed));
    }

    #[test]
    fn deserialize_unknown_variant_returns_error() {
        assert!(deserialize("INCREMENTAL_COMPRESSED").is_err());
        assert!(deserialize("incremental").is_err());
        assert!(deserialize("").is_err());
    }
}
