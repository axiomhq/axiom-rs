use serde::{Deserialize, Serialize};

/// A virtual field.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualField {
    pub id: String,
    pub dataset: String,
    pub name: String,
    pub description: String,
    pub expression: String,
}

/// The request to create or update a virtual field.
#[derive(Serialize, Debug, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct VirtualFieldCreateUpdateRequest {
    pub dataset: String,
    pub name: String,
    pub description: String,
    pub expression: String,
}

/// Sets the options for listing virtual fields.
#[derive(Serialize, Default)]
pub struct ListOptions {
    pub dataset: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
