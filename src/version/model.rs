use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct Version {
    #[serde(rename = "currentVersion")]
    pub current_version: String,
}
