use std::marker::PhantomData;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::Error;
/// An annotation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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
    pub time: chrono::DateTime<Utc>,
    ///End time of the annotation
    pub end_time: Option<chrono::DateTime<Utc>>,
}
/// An authenticated Axiom user.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct AnnotationRequest {
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens. For example, "production-deployment".
    #[serde(rename = "type")]
    annotation_type: String,
    /// Dataset names for which the annotation appears on charts
    datasets: Vec<String>,
    /// Explanation of the event the annotation marks on the charts
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Summary of the annotation that appears on the charts
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    /// URL relevant for the event marked by the annotation. For example, link to GitHub pull request.
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
    /// Time the annotation marks on the charts. If you don't include this field, Axiom assigns the time of the API request to the annotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<chrono::DateTime<Utc>>,
    ///End time of the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<chrono::DateTime<Utc>>,
}

impl AnnotationRequest {
    /// New annotation builder.
    pub fn builder() -> AnnotationBuilder<NeedsType> {
        AnnotationBuilder::default()
    }
    /// Helper to quickly create a simple annotation request with just a `type` and `datasets`.
    pub fn new(annotation_type: &impl ToString, datasets: Vec<String>) -> Self {
        AnnotationRequest::builder()
            .with_type(annotation_type)
            .with_datasets(datasets)
            .build()
    }
}

/// The builder needs an annotation type to be set.
pub struct NeedsType;
/// The builder needs datasets to be set.
pub struct NeedsDatasets;
/// The builder is ready to build the request but optional fields can still be set.
pub struct Optionals;

/// A builder for creating an annotation request.
#[derive(PartialEq, Eq, Debug)]
#[must_use]
pub struct AnnotationBuilder<T> {
    request: AnnotationRequest,
    _p: PhantomData<T>,
}

impl Default for AnnotationBuilder<NeedsType> {
    fn default() -> Self {
        Self {
            request: AnnotationRequest {
                annotation_type: String::new(),
                datasets: Vec::new(),
                description: None,
                title: None,
                url: None,
                time: None,
                end_time: None,
            },
            _p: PhantomData,
        }
    }
}

impl AnnotationBuilder<NeedsType> {
    /// Set the type of the annotation.
    ///
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens.
    /// For example, "production-deployment".
    pub fn with_type(self, annotation_type: &impl ToString) -> AnnotationBuilder<NeedsDatasets> {
        AnnotationBuilder {
            request: AnnotationRequest {
                annotation_type: annotation_type.to_string(),
                ..self.request
            },
            _p: PhantomData,
        }
    }
}

impl AnnotationBuilder<NeedsDatasets> {
    /// Set the datasets for which the annotation appears on charts.
    pub fn with_datasets(self, datasets: Vec<String>) -> AnnotationBuilder<Optionals> {
        AnnotationBuilder {
            request: AnnotationRequest {
                datasets,
                ..self.request
            },
            _p: PhantomData,
        }
    }
}

impl AnnotationBuilder<Optionals> {
    /// Builds the request
    pub fn build(self) -> AnnotationRequest {
        self.request
    }

    /// Set the description of the annotation.
    ///
    /// Explanation of the event the annotation marks on the charts.
    pub fn with_description(self, description: &impl ToString) -> Self {
        Self {
            request: AnnotationRequest {
                description: Some(description.to_string()),
                ..self.request
            },
            _p: PhantomData,
        }
    }

    /// Set the title of the annotation.
    ///
    /// Summary of the annotation that appears on the charts
    pub fn with_title(self, title: &impl ToString) -> Self {
        Self {
            request: AnnotationRequest {
                title: Some(title.to_string()),
                ..self.request
            },
            _p: PhantomData,
        }
    }

    /// Set the URL of the annotation.
    ///
    /// URL relevant for the event marked by the annotation. For example, link to GitHub pull request.
    pub fn with_url(self, url: Url) -> Self {
        Self {
            request: AnnotationRequest {
                url: Some(url),
                ..self.request
            },
            _p: PhantomData,
        }
    }

    /// Set the (start) time of the annotation.
    ///
    /// Time the annotation marks on the charts. If you don't include this field,
    /// Axiom assigns the time of the API request to the annotation.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_time(self, time: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(end_time) = self.request.end_time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: AnnotationRequest {
                time: Some(time),
                ..self.request
            },
            _p: PhantomData,
        })
    }

    /// Set the end time of the annotation.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_end_time(self, end_time: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(time) = self.request.time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: AnnotationRequest {
                end_time: Some(end_time),
                ..self.request
            },
            _p: PhantomData,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
/// A request to all annotations
#[must_use]
pub struct ListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    datasets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<chrono::DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<chrono::DateTime<Utc>>,
}

impl ListRequest {
    /// New list request builder.
    pub fn builder() -> ListRequestBuilder {
        ListRequestBuilder::default()
    }
}

/// A builder for creating a list request.
#[derive(PartialEq, Eq, Debug, Default)]
#[must_use]
pub struct ListRequestBuilder {
    request: ListRequest,
}

impl ListRequestBuilder {
    /// Set the datasets for which the annotations are listed.
    pub fn with_datasets(self, datasets: Vec<String>) -> Self {
        Self {
            request: ListRequest {
                datasets: Some(datasets),
                ..self.request
            },
        }
    }

    /// Set the start time of the list.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_time(self, start: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(end) = self.request.end {
            if start > end {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: ListRequest {
                start: Some(start),
                ..self.request
            },
        })
    }

    /// Set the end time of list.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_end(self, end: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(start) = self.request.start {
            if start > end {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: ListRequest {
                end: Some(end),
                ..self.request
            },
        })
    }
    /// Builds the request
    pub fn build(self) -> ListRequest {
        self.request
    }
}

/// A request to update an annotation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct AnnotationUpdateRequest {
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens. For example, "production-deployment".
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation_type: Option<String>,
    /// Dataset names for which the annotation appears on charts
    #[serde(skip_serializing_if = "Option::is_none")]
    datasets: Option<Vec<String>>,
    /// Explanation of the event the annotation marks on the charts
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// Summary of the annotation that appears on the charts
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    /// URL relevant for the event marked by the annotation. For example, link to GitHub pull request.
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
    /// Time the annotation marks on the charts. If you don't include this field, Axiom assigns the time of the API request to the annotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<chrono::DateTime<Utc>>,
    ///End time of the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<chrono::DateTime<Utc>>,
}

/// A builder for creating an annotation request.
#[derive(PartialEq, Eq, Debug)]
#[must_use]
pub struct AnnotationUpdateBuilder {
    request: AnnotationUpdateRequest,
}

impl AnnotationUpdateBuilder {
    /// Builds the request
    ///
    /// # Errors
    /// If the request is empty.
    pub fn build(self) -> Result<AnnotationUpdateRequest, Error> {
        let request = self.request;
        if request.annotation_type.is_none()
            && request.datasets.is_none()
            && request.description.is_none()
            && request.title.is_none()
            && request.url.is_none()
            && request.time.is_none()
            && request.end_time.is_none()
        {
            return Err(Error::EmptyUpdate);
        }
        Ok(request)
    }

    /// Set the type of the annotation.
    ///
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens.
    /// For example, "production-deployment".
    pub fn with_type(self, annotation_type: &impl ToString) -> Self {
        AnnotationUpdateBuilder {
            request: AnnotationUpdateRequest {
                annotation_type: Some(annotation_type.to_string()),
                ..self.request
            },
        }
    }

    /// Set the datasets for which the annotation appears on charts.
    pub fn with_datasets(self, datasets: Vec<String>) -> Self {
        AnnotationUpdateBuilder {
            request: AnnotationUpdateRequest {
                datasets: Some(datasets),
                ..self.request
            },
        }
    }

    /// Set the description of the annotation.
    ///
    /// Explanation of the event the annotation marks on the charts.
    pub fn with_description(self, description: &impl ToString) -> Self {
        Self {
            request: AnnotationUpdateRequest {
                description: Some(description.to_string()),
                ..self.request
            },
        }
    }

    /// Set the title of the annotation.
    ///
    /// Summary of the annotation that appears on the charts
    pub fn with_title(self, title: &impl ToString) -> Self {
        Self {
            request: AnnotationUpdateRequest {
                title: Some(title.to_string()),
                ..self.request
            },
        }
    }

    /// Set the URL of the annotation.
    ///
    /// URL relevant for the event marked by the annotation. For example, link to GitHub pull request.
    pub fn with_url(self, url: Url) -> Self {
        Self {
            request: AnnotationUpdateRequest {
                url: Some(url),
                ..self.request
            },
        }
    }

    /// Set the (start) time of the annotation.
    ///
    /// Time the annotation marks on the charts. If you don't include this field,
    /// Axiom assigns the time of the API request to the annotation.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_time(self, time: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(end_time) = self.request.end_time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: AnnotationUpdateRequest {
                time: Some(time),
                ..self.request
            },
        })
    }

    /// Set the end time of the annotation.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_end_time(self, end_time: chrono::DateTime<Utc>) -> Result<Self, Error> {
        if let Some(time) = self.request.time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: AnnotationUpdateRequest {
                end_time: Some(end_time),
                ..self.request
            },
        })
    }
}
