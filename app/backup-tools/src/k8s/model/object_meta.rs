use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ObjectMeta {
    pub name: String,
    pub namespace: Option<String>,
}
