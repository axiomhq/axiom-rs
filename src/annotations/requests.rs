//! Request types for the annotations API.

use crate::Error;
use chrono::FixedOffset;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use url::Url;

/// A request to create an annotation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct Create {
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
    time: Option<chrono::DateTime<FixedOffset>>,
    ///End time of the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<chrono::DateTime<FixedOffset>>,
}

impl Create {
    /// New annotation builder.
    pub fn builder() -> CreateBuilder<NeedsType> {
        CreateBuilder {
            request: Create {
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
    /// Helper to quickly create a simple annotation request with just a `type` and `datasets`.
    ///
    /// # Errors
    /// If the datasets are empty.
    /// If the annotation type is empty.
    pub fn new(
        annotation_type: &(impl ToString + ?Sized),
        datasets: Vec<String>,
    ) -> Result<Self, Error> {
        Ok(Create::builder()
            .with_type(annotation_type)?
            .with_datasets(datasets)?
            .build())
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
pub struct CreateBuilder<T> {
    request: Create,
    _p: PhantomData<T>,
}

impl CreateBuilder<NeedsType> {
    /// Set the type of the annotation.
    ///
    /// Type of the event marked by the annotation. Use only alphanumeric characters or hyphens.
    /// For example, "production-deployment".
    ///
    /// # Errors
    /// If the type is empty.
    pub fn with_type(
        self,
        annotation_type: &(impl ToString + ?Sized),
    ) -> Result<CreateBuilder<NeedsDatasets>, Error> {
        let annotation_type = annotation_type.to_string();
        if annotation_type.is_empty() {
            return Err(Error::EmptyType);
        }
        Ok(CreateBuilder {
            request: Create {
                annotation_type,
                ..self.request
            },
            _p: PhantomData,
        })
    }
}

impl CreateBuilder<NeedsDatasets> {
    /// Set the datasets for which the annotation appears on charts.
    ///
    /// # Errors
    /// If the datasets are empty.
    pub fn with_datasets(self, datasets: Vec<String>) -> Result<CreateBuilder<Optionals>, Error> {
        if datasets.is_empty() {
            return Err(Error::EmptyDatasets);
        }
        Ok(CreateBuilder {
            request: Create {
                datasets,
                ..self.request
            },
            _p: PhantomData,
        })
    }
}

impl CreateBuilder<Optionals> {
    /// Builds the request
    pub fn build(self) -> Create {
        self.request
    }

    /// Set the description of the annotation.
    ///
    /// Explanation of the event the annotation marks on the charts.
    pub fn with_description(self, description: &(impl ToString + ?Sized)) -> Self {
        Self {
            request: Create {
                description: Some(description.to_string()),
                ..self.request
            },
            _p: PhantomData,
        }
    }

    /// Set the title of the annotation.
    ///
    /// Summary of the annotation that appears on the charts
    pub fn with_title(self, title: &(impl ToString + ?Sized)) -> Self {
        Self {
            request: Create {
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
            request: Create {
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
    pub fn with_time(self, time: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(end_time) = self.request.end_time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: Create {
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
    pub fn with_end_time(self, end_time: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(time) = self.request.time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: Create {
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
pub struct List {
    #[serde(skip_serializing_if = "Option::is_none")]
    datasets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<chrono::DateTime<FixedOffset>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<chrono::DateTime<FixedOffset>>,
}

impl List {
    /// New list request builder.
    pub fn builder() -> ListBuilder {
        ListBuilder::default()
    }
}

/// A builder for creating a list request.
#[derive(PartialEq, Eq, Debug, Default)]
#[must_use]
pub struct ListBuilder {
    request: List,
}

impl ListBuilder {
    /// Set the datasets for which the annotations are listed.
    pub fn with_datasets(self, datasets: Vec<String>) -> Self {
        Self {
            request: List {
                datasets: Some(datasets),
                ..self.request
            },
        }
    }

    /// Set the start time of the list.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_start(self, start: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(end) = self.request.end {
            if start > end {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: List {
                start: Some(start),
                ..self.request
            },
        })
    }

    /// Set the end time of list.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_end(self, end: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(start) = self.request.start {
            if start > end {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: List {
                end: Some(end),
                ..self.request
            },
        })
    }
    /// Builds the request
    pub fn build(self) -> List {
        self.request
    }
}

/// A request to update an annotation.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[must_use]
pub struct Update {
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
    time: Option<chrono::DateTime<FixedOffset>>,
    ///End time of the annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<chrono::DateTime<FixedOffset>>,
}

impl Update {
    /// New update builder.
    pub fn builder() -> UpdateBuilder {
        UpdateBuilder {
            request: Update {
                annotation_type: None,
                datasets: None,
                description: None,
                title: None,
                url: None,
                time: None,
                end_time: None,
            },
        }
    }
}

/// A builder for creating an annotation request.
#[derive(PartialEq, Eq, Debug)]
#[must_use]
pub struct UpdateBuilder {
    request: Update,
}

impl UpdateBuilder {
    /// Builds the request
    ///
    /// # Errors
    /// If the request is empty.
    pub fn build(self) -> Result<Update, Error> {
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
    ///
    /// # Errors
    /// If the type is empty.
    pub fn with_type(self, annotation_type: &(impl ToString + ?Sized)) -> Result<Self, Error> {
        let annotation_type = annotation_type.to_string();
        if annotation_type.is_empty() {
            return Err(Error::EmptyType);
        }
        Ok(UpdateBuilder {
            request: Update {
                annotation_type: Some(annotation_type),
                ..self.request
            },
        })
    }

    /// Set the datasets for which the annotation appears on charts.
    pub fn with_datasets(self, datasets: Vec<String>) -> Self {
        UpdateBuilder {
            request: Update {
                datasets: Some(datasets),
                ..self.request
            },
        }
    }

    /// Set the description of the annotation.
    ///
    /// Explanation of the event the annotation marks on the charts.
    pub fn with_description(self, description: &(impl ToString + ?Sized)) -> Self {
        Self {
            request: Update {
                description: Some(description.to_string()),
                ..self.request
            },
        }
    }

    /// Set the title of the annotation.
    ///
    /// Summary of the annotation that appears on the charts
    pub fn with_title(self, title: &(impl ToString + ?Sized)) -> Self {
        Self {
            request: Update {
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
            request: Update {
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
    pub fn with_time(self, time: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(end_time) = self.request.end_time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: Update {
                time: Some(time),
                ..self.request
            },
        })
    }

    /// Set the end time of the annotation.
    ///
    /// # Errors
    /// If the start time is after the end time.
    pub fn with_end_time(self, end_time: chrono::DateTime<FixedOffset>) -> Result<Self, Error> {
        if let Some(time) = self.request.time {
            if time > end_time {
                return Err(Error::InvalidTimeOrder);
            }
        }
        Ok(Self {
            request: Update {
                end_time: Some(end_time),
                ..self.request
            },
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_datasets() {
        let res = super::Create::new("snot", vec![]);
        assert!(matches!(res, Err(super::Error::EmptyDatasets)));

        let res = super::Create::builder()
            .with_type("snot")
            .expect("we got type")
            .with_datasets(vec![]);
        assert!(matches!(res, Err(super::Error::EmptyDatasets)));
    }
    #[test]
    fn create_invalid_times() {
        let start = chrono::DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("the time is right");
        let end = chrono::DateTime::parse_from_rfc3339("2023-02-06T11:39:28.382Z")
            .expect("the time is right");
        let res = super::Create::builder()
            .with_type("snot")
            .expect("we got type")
            .with_datasets(vec!["badger".to_string()])
            .expect("we got datasets")
            .with_time(start)
            .expect("we got time")
            .with_end_time(end);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
        let res = super::Create::builder()
            .with_type("snot")
            .expect("we got type")
            .with_datasets(vec!["badger".to_string()])
            .expect("we got datasets")
            .with_end_time(end)
            .expect("we got time")
            .with_time(start);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
    }

    #[test]
    fn list_invalid_times() {
        let start = chrono::DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("the time is right");
        let end = chrono::DateTime::parse_from_rfc3339("2023-02-06T11:39:28.382Z")
            .expect("the time is right");
        let res = super::List::builder()
            .with_start(start)
            .expect("we got start")
            .with_end(end);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
        let res = super::List::builder()
            .with_end(end)
            .expect("we got start")
            .with_start(start);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
    }

    #[test]
    fn update_invalid_times() {
        let start = chrono::DateTime::parse_from_rfc3339("2024-02-06T11:39:28.382Z")
            .expect("the time is right");
        let end = chrono::DateTime::parse_from_rfc3339("2023-02-06T11:39:28.382Z")
            .expect("the time is right");
        let res = super::Update::builder()
            .with_time(start)
            .expect("we got start")
            .with_end_time(end);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
        let res = super::Update::builder()
            .with_end_time(end)
            .expect("we got start")
            .with_time(start);
        assert!(matches!(res, Err(super::Error::InvalidTimeOrder)));
    }
}
