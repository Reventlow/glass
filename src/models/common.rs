//! Common types shared across SDP API models.
//!
//! This module defines pagination, response wrappers, and other
//! shared types used by multiple API endpoints.

use serde::{Deserialize, Serialize};

use crate::error::GlassError;

/// Pagination and sorting parameters for list operations.
///
/// Used in `input_data` to control the number of results returned
/// and their ordering.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListInfo {
    /// Maximum number of rows to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u32>,

    /// Starting index (0-based) for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,

    /// Field to sort by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_field: Option<String>,

    /// Sort order: "asc" or "desc".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,

    /// Whether to get only the row count without data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_total_count: Option<bool>,
}

impl ListInfo {
    /// Creates a new ListInfo with default pagination.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of rows to return.
    pub fn with_row_count(mut self, count: u32) -> Self {
        self.row_count = Some(count);
        self
    }

    /// Sets the starting index for pagination.
    pub fn with_start_index(mut self, index: u32) -> Self {
        self.start_index = Some(index);
        self
    }

    /// Requests the total count along with results.
    pub fn with_total_count(mut self) -> Self {
        self.get_total_count = Some(true);
        self
    }
}

/// A single search criterion for filtering list results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCriterion {
    /// The field to filter on.
    pub field: String,

    /// The condition: "is", "is not", "contains", etc.
    pub condition: String,

    /// The value(s) to match.
    pub value: serde_json::Value,

    /// Logical operator to combine with next criterion: "AND" or "OR".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_operator: Option<String>,
}

impl SearchCriterion {
    /// Creates an "is" condition for exact matching.
    pub fn is(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            condition: "is".to_string(),
            value: serde_json::Value::String(value.into()),
            logical_operator: None,
        }
    }

    /// Creates a "contains" condition for partial matching.
    pub fn contains(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            condition: "contains".to_string(),
            value: serde_json::Value::String(value.into()),
            logical_operator: None,
        }
    }

    /// Adds an AND operator to chain with the next criterion.
    pub fn and(mut self) -> Self {
        self.logical_operator = Some("AND".to_string());
        self
    }

    /// Adds an OR operator to chain with the next criterion.
    pub fn or(mut self) -> Self {
        self.logical_operator = Some("OR".to_string());
        self
    }
}

/// Wrapper for search criteria in list requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchCriteria {
    /// List of search criteria to apply.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<SearchCriterion>,
}

impl SearchCriteria {
    /// Creates empty search criteria.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a search criterion.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, criterion: SearchCriterion) -> Self {
        self.criteria.push(criterion);
        self
    }

    /// Returns true if there are no criteria.
    pub fn is_empty(&self) -> bool {
        self.criteria.is_empty()
    }
}

/// Response status from SDP API.
///
/// Every SDP API response includes this status block to indicate
/// success or failure.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseStatus {
    /// Status code: 2000 for success, 4000+ for errors.
    pub status_code: u32,

    /// Status string: "success" or "failed".
    #[serde(default)]
    pub status: String,

    /// Error messages (present on failure).
    #[serde(default)]
    pub messages: Vec<ResponseMessage>,
}

/// A single message in the response status.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMessage {
    /// The error or info message.
    #[serde(default)]
    pub message: String,

    /// Status code for this specific message.
    #[serde(default)]
    pub status_code: Option<u32>,

    /// Type of message.
    #[serde(rename = "type", default)]
    pub message_type: Option<String>,
}

impl ResponseStatus {
    /// Returns true if the response indicates success.
    pub fn is_success(&self) -> bool {
        self.status_code == 2000
    }

    /// Converts a failed response status into a GlassError.
    pub fn into_error(self) -> GlassError {
        let message = self
            .messages
            .first()
            .map(|m| m.message.clone())
            .unwrap_or_else(|| "Unknown error".to_string());

        // Check for specific error codes
        match self.status_code {
            4001 => GlassError::Authentication,
            4005 => GlassError::NotFound {
                id: "unknown".to_string(),
            },
            _ => GlassError::SdpApi {
                code: self.status_code,
                message,
                request_id: None,
            },
        }
    }
}

/// Generic wrapper for SDP API responses.
///
/// SDP API responses follow a consistent envelope pattern with
/// `response_status` and the actual data in a field matching the
/// resource type.
#[derive(Debug, Clone, Deserialize)]
pub struct SdpResponse<T> {
    /// Response status indicating success or failure.
    pub response_status: ResponseStatus,

    /// The actual response data (when successful).
    #[serde(flatten)]
    pub data: T,
}

impl<T> SdpResponse<T> {
    /// Converts the response into a Result, checking the status.
    pub fn into_result(self) -> Result<T, GlassError> {
        if self.response_status.is_success() {
            Ok(self.data)
        } else {
            Err(self.response_status.into_error())
        }
    }
}

/// Response wrapper for list operations that includes requests array.
#[derive(Debug, Clone, Deserialize)]
pub struct ListRequestsResponse {
    /// List of request summaries.
    #[serde(default)]
    pub requests: Vec<super::RequestSummary>,

    /// Pagination info (if requested).
    #[serde(default)]
    pub list_info: Option<ListInfoResponse>,
}

/// Pagination info returned in list responses.
#[derive(Debug, Clone, Deserialize)]
pub struct ListInfoResponse {
    /// Whether there are more results.
    #[serde(default)]
    pub has_more_rows: bool,

    /// Total count of matching records (if requested).
    #[serde(default)]
    pub total_count: Option<u32>,

    /// Starting index of this page.
    #[serde(default)]
    pub start_index: Option<u32>,

    /// Number of rows returned.
    #[serde(default)]
    pub row_count: Option<u32>,
}

/// Response wrapper for single request operations.
#[derive(Debug, Clone, Deserialize)]
pub struct GetRequestResponse {
    /// The full request details.
    pub request: super::Request,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_info_builder() {
        let info = ListInfo::new().with_row_count(10).with_start_index(20);
        assert_eq!(info.row_count, Some(10));
        assert_eq!(info.start_index, Some(20));
    }

    #[test]
    fn test_search_criterion_is() {
        let criterion = SearchCriterion::is("status.name", "Open");
        assert_eq!(criterion.field, "status.name");
        assert_eq!(criterion.condition, "is");
    }

    #[test]
    fn test_response_status_success() {
        let status = ResponseStatus {
            status_code: 2000,
            status: "success".to_string(),
            messages: vec![],
        };
        assert!(status.is_success());
    }

    #[test]
    fn test_response_status_failure() {
        let status = ResponseStatus {
            status_code: 4000,
            status: "failed".to_string(),
            messages: vec![ResponseMessage {
                message: "Invalid input".to_string(),
                status_code: Some(4000),
                message_type: Some("error".to_string()),
            }],
        };
        assert!(!status.is_success());
        let err = status.into_error();
        assert!(matches!(err, GlassError::SdpApi { code: 4000, .. }));
    }
}
