//! Request (ticket) models for ServiceDesk Plus API.
//!
//! This module defines the data structures for SDP requests/tickets,
//! including both summary (list) and full detail variants.

use serde::{Deserialize, Serialize};

/// A named entity reference used throughout SDP API.
///
/// Many SDP fields reference other entities by ID and name,
/// using this consistent structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedEntity {
    /// Unique identifier.
    #[serde(default)]
    pub id: Option<String>,

    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
}

impl NamedEntity {
    /// Returns the name if present, otherwise a placeholder.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }
}

/// Timestamp representation from SDP API.
///
/// SDP returns timestamps with both a numeric value (epoch milliseconds)
/// and a human-readable display_value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpTimestamp {
    /// Epoch milliseconds (can be string or integer in API response).
    #[serde(default, deserialize_with = "deserialize_optional_string_or_int")]
    pub value: Option<String>,

    /// Human-readable format.
    #[serde(default)]
    pub display_value: Option<String>,
}

/// Deserializes an optional value that can be either a string or an integer into Option<String>.
fn deserialize_optional_string_or_int<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct OptionalStringOrIntVisitor;

    impl<'de> Visitor<'de> for OptionalStringOrIntVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null, a string, or an integer")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(StringOrIntVisitor).map(Some)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }
    }

    struct StringOrIntVisitor;

    impl<'de> Visitor<'de> for StringOrIntVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or an integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(OptionalStringOrIntVisitor)
}

impl SdpTimestamp {
    /// Returns the display value if present, otherwise the raw value.
    pub fn display(&self) -> Option<&str> {
        self.display_value
            .as_deref()
            .or(self.value.as_deref())
    }
}

/// Summary of a request for list operations.
///
/// This is a lighter-weight representation returned when listing
/// requests, containing only the most commonly needed fields.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestSummary {
    /// Unique request ID.
    pub id: String,

    /// Subject/title of the request.
    #[serde(default)]
    pub subject: Option<String>,

    /// Current status.
    #[serde(default)]
    pub status: Option<NamedEntity>,

    /// Priority level.
    #[serde(default)]
    pub priority: Option<NamedEntity>,

    /// Assigned technician.
    #[serde(default)]
    pub technician: Option<NamedEntity>,

    /// Requester who created the ticket.
    #[serde(default)]
    pub requester: Option<NamedEntity>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_time: Option<SdpTimestamp>,

    /// Last update timestamp.
    #[serde(default)]
    pub last_updated_time: Option<SdpTimestamp>,

    /// Due date/time.
    #[serde(default)]
    pub due_by_time: Option<SdpTimestamp>,

    /// Request type (Incident, Service Request, etc.).
    #[serde(default)]
    pub request_type: Option<NamedEntity>,

    /// Category.
    #[serde(default)]
    pub category: Option<NamedEntity>,

    /// Subcategory.
    #[serde(default)]
    pub subcategory: Option<NamedEntity>,

    /// Site/location.
    #[serde(default)]
    pub site: Option<NamedEntity>,

    /// Group the request is assigned to.
    #[serde(default)]
    pub group: Option<NamedEntity>,
}

impl RequestSummary {
    /// Returns the subject or a placeholder.
    pub fn display_subject(&self) -> &str {
        self.subject.as_deref().unwrap_or("(No subject)")
    }

    /// Returns the status name or "Unknown".
    pub fn display_status(&self) -> &str {
        self.status
            .as_ref()
            .and_then(|s| s.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the priority name or "Unknown".
    pub fn display_priority(&self) -> &str {
        self.priority
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the technician name or "Unassigned".
    pub fn display_technician(&self) -> &str {
        self.technician
            .as_ref()
            .and_then(|t| t.name.as_deref())
            .unwrap_or("Unassigned")
    }

    /// Returns the requester name or "Unknown".
    pub fn display_requester(&self) -> &str {
        self.requester
            .as_ref()
            .and_then(|r| r.name.as_deref())
            .unwrap_or("Unknown")
    }
}

/// Full details of a single request.
///
/// This is the complete representation returned when fetching
/// a specific request by ID.
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    /// Unique request ID.
    pub id: String,

    /// Subject/title of the request.
    #[serde(default)]
    pub subject: Option<String>,

    /// Description/body of the request (may contain HTML).
    #[serde(default)]
    pub description: Option<String>,

    /// Current status.
    #[serde(default)]
    pub status: Option<NamedEntity>,

    /// Priority level.
    #[serde(default)]
    pub priority: Option<NamedEntity>,

    /// Urgency level.
    #[serde(default)]
    pub urgency: Option<NamedEntity>,

    /// Impact level.
    #[serde(default)]
    pub impact: Option<NamedEntity>,

    /// Assigned technician.
    #[serde(default)]
    pub technician: Option<NamedEntity>,

    /// Requester who created the ticket.
    #[serde(default)]
    pub requester: Option<NamedEntity>,

    /// Request type (Incident, Service Request, etc.).
    #[serde(default)]
    pub request_type: Option<NamedEntity>,

    /// Category.
    #[serde(default)]
    pub category: Option<NamedEntity>,

    /// Subcategory.
    #[serde(default)]
    pub subcategory: Option<NamedEntity>,

    /// Item (third level categorization).
    #[serde(default)]
    pub item: Option<NamedEntity>,

    /// Site/location.
    #[serde(default)]
    pub site: Option<NamedEntity>,

    /// Group the request is assigned to.
    #[serde(default)]
    pub group: Option<NamedEntity>,

    /// Level (support tier).
    #[serde(default)]
    pub level: Option<NamedEntity>,

    /// Mode of request creation.
    #[serde(default)]
    pub mode: Option<NamedEntity>,

    /// Service associated with the request.
    #[serde(default)]
    pub service: Option<NamedEntity>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_time: Option<SdpTimestamp>,

    /// Last update timestamp.
    #[serde(default)]
    pub last_updated_time: Option<SdpTimestamp>,

    /// Due date/time.
    #[serde(default)]
    pub due_by_time: Option<SdpTimestamp>,

    /// First response due time.
    #[serde(default)]
    pub first_response_due_by_time: Option<SdpTimestamp>,

    /// Resolution due time.
    #[serde(default)]
    pub resolution_due_by_time: Option<SdpTimestamp>,

    /// Completed time.
    #[serde(default)]
    pub completed_time: Option<SdpTimestamp>,

    /// Resolution details.
    #[serde(default)]
    pub resolution: Option<Resolution>,

    /// Closure information.
    #[serde(default)]
    pub closure_info: Option<ClosureInfo>,

    /// Whether the request is overdue.
    #[serde(default)]
    pub is_overdue: Option<bool>,

    /// Whether the request is marked as first call resolution.
    #[serde(default)]
    pub is_fcr: Option<bool>,

    /// Has attachments.
    #[serde(default)]
    pub has_attachments: Option<bool>,

    /// Has notes.
    #[serde(default)]
    pub has_notes: Option<bool>,

    /// Email IDs related to this request.
    #[serde(default)]
    pub email_ids_to_notify: Option<Vec<String>>,

    /// Approval status.
    #[serde(default)]
    pub approval_status: Option<NamedEntity>,
}

impl Request {
    /// Returns the subject or a placeholder.
    pub fn display_subject(&self) -> &str {
        self.subject.as_deref().unwrap_or("(No subject)")
    }

    /// Returns the status name or "Unknown".
    pub fn display_status(&self) -> &str {
        self.status
            .as_ref()
            .and_then(|s| s.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the priority name or "Unknown".
    pub fn display_priority(&self) -> &str {
        self.priority
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the technician name or "Unassigned".
    pub fn display_technician(&self) -> &str {
        self.technician
            .as_ref()
            .and_then(|t| t.name.as_deref())
            .unwrap_or("Unassigned")
    }

    /// Returns the requester name or "Unknown".
    pub fn display_requester(&self) -> &str {
        self.requester
            .as_ref()
            .and_then(|r| r.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the group name if assigned.
    pub fn display_group(&self) -> Option<&str> {
        self.group.as_ref().and_then(|g| g.name.as_deref())
    }

    /// Returns the category path (category > subcategory > item).
    pub fn category_path(&self) -> String {
        let parts: Vec<&str> = [
            self.category.as_ref().and_then(|c| c.name.as_deref()),
            self.subcategory.as_ref().and_then(|c| c.name.as_deref()),
            self.item.as_ref().and_then(|c| c.name.as_deref()),
        ]
        .into_iter()
        .flatten()
        .collect();

        if parts.is_empty() {
            "Uncategorized".to_string()
        } else {
            parts.join(" > ")
        }
    }
}

/// Resolution details for a completed request.
#[derive(Debug, Clone, Deserialize)]
pub struct Resolution {
    /// Resolution content (may contain HTML).
    #[serde(default)]
    pub content: Option<String>,

    /// Who submitted the resolution.
    #[serde(default)]
    pub submitted_by: Option<NamedEntity>,

    /// When the resolution was submitted.
    #[serde(default)]
    pub submitted_on: Option<SdpTimestamp>,
}

/// Closure information for a closed request.
#[derive(Debug, Clone, Deserialize)]
pub struct ClosureInfo {
    /// Closure code.
    #[serde(default)]
    pub closure_code: Option<NamedEntity>,

    /// Closure comments.
    #[serde(default)]
    pub closure_comments: Option<String>,

    /// Who closed the request.
    #[serde(default)]
    pub closed_by: Option<NamedEntity>,

    /// When the request was closed.
    #[serde(default)]
    pub closed_time: Option<SdpTimestamp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_entity_display() {
        let entity = NamedEntity {
            id: Some("1".to_string()),
            name: Some("Test Name".to_string()),
        };
        assert_eq!(entity.display_name(), "Test Name");

        let empty = NamedEntity {
            id: None,
            name: None,
        };
        assert_eq!(empty.display_name(), "Unknown");
    }

    #[test]
    fn test_sdp_timestamp_display() {
        let ts = SdpTimestamp {
            value: Some("1706745600000".to_string()),
            display_value: Some("Feb 1, 2024".to_string()),
        };
        assert_eq!(ts.display(), Some("Feb 1, 2024"));

        let ts_value_only = SdpTimestamp {
            value: Some("1706745600000".to_string()),
            display_value: None,
        };
        assert_eq!(ts_value_only.display(), Some("1706745600000"));
    }

    #[test]
    fn test_request_summary_display_methods() {
        let summary = RequestSummary {
            id: "123".to_string(),
            subject: Some("Test Subject".to_string()),
            status: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("Open".to_string()),
            }),
            priority: Some(NamedEntity {
                id: Some("2".to_string()),
                name: Some("High".to_string()),
            }),
            technician: None,
            requester: Some(NamedEntity {
                id: Some("3".to_string()),
                name: Some("John Doe".to_string()),
            }),
            created_time: None,
            last_updated_time: None,
            due_by_time: None,
            request_type: None,
            category: None,
            subcategory: None,
            site: None,
            group: None,
        };

        assert_eq!(summary.display_subject(), "Test Subject");
        assert_eq!(summary.display_status(), "Open");
        assert_eq!(summary.display_priority(), "High");
        assert_eq!(summary.display_technician(), "Unassigned");
        assert_eq!(summary.display_requester(), "John Doe");
    }

    #[test]
    fn test_request_category_path() {
        let request = Request {
            id: "123".to_string(),
            subject: None,
            description: None,
            status: None,
            priority: None,
            urgency: None,
            impact: None,
            technician: None,
            requester: None,
            request_type: None,
            category: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("Hardware".to_string()),
            }),
            subcategory: Some(NamedEntity {
                id: Some("2".to_string()),
                name: Some("Laptop".to_string()),
            }),
            item: Some(NamedEntity {
                id: Some("3".to_string()),
                name: Some("Screen".to_string()),
            }),
            site: None,
            group: None,
            level: None,
            mode: None,
            service: None,
            created_time: None,
            last_updated_time: None,
            due_by_time: None,
            first_response_due_by_time: None,
            resolution_due_by_time: None,
            completed_time: None,
            resolution: None,
            closure_info: None,
            is_overdue: None,
            is_fcr: None,
            has_attachments: None,
            has_notes: None,
            email_ids_to_notify: None,
            approval_status: None,
        };

        assert_eq!(request.category_path(), "Hardware > Laptop > Screen");
    }
}
