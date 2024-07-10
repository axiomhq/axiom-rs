use chrono::FixedOffset;
use serde::{Deserialize, Serialize};
use url::Url;

/// An annotation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]

pub struct Annotation {
    /// Unique ID of the annotation
    pub id: String,
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens. For example, "production-deployment".
    #[serde(rename = "type")]
    pub annotation_type: String,
    /// Dataset names for which the annotation appears on charts
    pub datasets: Vec<String>,
    /// Explanation of the event the annotation marks on the charts
    pub description: Option<String>,
    /// Summary of the annotation that appears on the charts
    pub title: Option<String>,
    /// URL relevant for the event marked by the annotation. For example, link to GitHub pull request.
    pub url: Option<Url>,
    /// Time the annotation marks on the charts. If you don't include this field, Axiom assigns the time of the API request to the annotation.
    pub time: chrono::DateTime<FixedOffset>,
    ///End time of the annotation
    pub end_time: Option<chrono::DateTime<FixedOffset>>,
}
