//! Tool input parameter structs for MCP tools.
//!
//! This module defines the input types for each MCP tool, with
//! JSON Schema derivation for MCP tool discovery.
//!
//! # Input Sanitization
//!
//! All input structs implement `sanitize()` which trims whitespace
//! from string fields. This should be called before processing input.

use rmcp::schemars::{self, JsonSchema};
use serde::Deserialize;

/// Helper function to trim an optional string.
fn trim_option(s: &Option<String>) -> Option<String> {
    s.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Input parameters for the list_requests tool.
///
/// All fields are optional - use them to filter the results.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListRequestsInput {
    /// Filter by ticket status (e.g., "Åben", "Tildelt", "I gang", "Lukket").
    #[serde(default)]
    pub status: Option<String>,

    /// Filter by priority level (e.g., "Low", "Medium", "High", "Urgent").
    #[serde(default)]
    pub priority: Option<String>,

    /// Filter by assigned technician name (e.g., "Gorm Reventlow").
    #[serde(default)]
    pub technician: Option<String>,

    /// Filter by requester name (e.g., "Henriette Meissner").
    #[serde(default)]
    pub requester: Option<String>,

    /// If true, only return open tickets (excludes Lukket, Annulleret, Udført statuses).
    #[serde(default)]
    pub open_only: Option<bool>,

    /// Filter tickets created after this date (ISO 8601 format: YYYY-MM-DD).
    #[serde(default)]
    pub created_after: Option<String>,

    /// Filter tickets created before this date (ISO 8601 format: YYYY-MM-DD).
    #[serde(default)]
    pub created_before: Option<String>,

    /// Maximum number of tickets to return (default: 20, max: 100).
    #[serde(default)]
    pub limit: Option<u32>,

    /// Number of tickets to skip for pagination (default: 0).
    #[serde(default)]
    pub offset: Option<u32>,
}

impl ListRequestsInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            status: trim_option(&self.status),
            priority: trim_option(&self.priority),
            technician: trim_option(&self.technician),
            requester: trim_option(&self.requester),
            open_only: self.open_only,
            created_after: trim_option(&self.created_after),
            created_before: trim_option(&self.created_before),
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Input parameters for the get_request tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetRequestInput {
    /// The unique ID of the ticket to retrieve.
    pub request_id: String,
}

impl GetRequestInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            request_id: self.request_id.trim().to_string(),
        }
    }
}

/// Input parameters for the list_technicians tool.
///
/// All fields are optional.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListTechniciansInput {
    /// Filter technicians by support group name.
    #[serde(default)]
    pub group: Option<String>,

    /// Maximum number of technicians to return (default: 50).
    #[serde(default)]
    pub limit: Option<u32>,
}

impl ListTechniciansInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            group: trim_option(&self.group),
            limit: self.limit,
        }
    }
}

// ============================================================================
// Write operation input structs (M4)
// ============================================================================

/// Input parameters for the create_request tool.
///
/// Subject is required. All other fields are optional.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CreateRequestInput {
    /// Ticket subject/title (required, max 250 characters).
    pub subject: String,

    /// Detailed description of the issue or request (supports HTML).
    #[serde(default)]
    pub description: Option<String>,

    /// Email address of the person reporting the issue.
    #[serde(default)]
    pub requester_email: Option<String>,

    /// Priority level: 'Low', 'Medium', 'High', or 'Urgent'.
    #[serde(default)]
    pub priority: Option<String>,

    /// Category name for the ticket (e.g., 'Hardware', 'Software', 'Network').
    #[serde(default)]
    pub category: Option<String>,

    /// Subcategory name (must be valid for the chosen category).
    #[serde(default)]
    pub subcategory: Option<String>,

    /// Item name (must be valid for the chosen subcategory).
    #[serde(default)]
    pub item: Option<String>,

    /// Support group to assign the ticket to.
    #[serde(default)]
    pub group: Option<String>,

    /// ID of technician to assign (use list_technicians to find IDs).
    #[serde(default)]
    pub technician_id: Option<String>,
}

impl CreateRequestInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            subject: self.subject.trim().to_string(),
            description: trim_option(&self.description),
            requester_email: trim_option(&self.requester_email),
            priority: trim_option(&self.priority),
            category: trim_option(&self.category),
            subcategory: trim_option(&self.subcategory),
            item: trim_option(&self.item),
            group: trim_option(&self.group),
            technician_id: trim_option(&self.technician_id),
        }
    }
}

/// Input parameters for the update_request tool.
///
/// Request ID is required. At least one other field must be provided.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct UpdateRequestInput {
    /// The unique ID of the ticket to update.
    pub request_id: String,

    /// New subject/title for the ticket (max 250 characters).
    #[serde(default)]
    pub subject: Option<String>,

    /// Updated description (supports HTML).
    #[serde(default)]
    pub description: Option<String>,

    /// New priority level: 'Low', 'Medium', 'High', or 'Urgent'.
    #[serde(default)]
    pub priority: Option<String>,

    /// New status (e.g., 'Open', 'In Progress', 'On Hold', 'Resolved').
    #[serde(default)]
    pub status: Option<String>,

    /// New category name.
    #[serde(default)]
    pub category: Option<String>,

    /// New subcategory name.
    #[serde(default)]
    pub subcategory: Option<String>,

    /// New support group.
    #[serde(default)]
    pub group: Option<String>,

    /// ID of technician to reassign to.
    #[serde(default)]
    pub technician_id: Option<String>,
}

impl UpdateRequestInput {
    /// Returns true if at least one field besides request_id is set.
    pub fn has_updates(&self) -> bool {
        self.subject.is_some()
            || self.description.is_some()
            || self.priority.is_some()
            || self.status.is_some()
            || self.category.is_some()
            || self.subcategory.is_some()
            || self.group.is_some()
            || self.technician_id.is_some()
    }

    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            request_id: self.request_id.trim().to_string(),
            subject: trim_option(&self.subject),
            description: trim_option(&self.description),
            priority: trim_option(&self.priority),
            status: trim_option(&self.status),
            category: trim_option(&self.category),
            subcategory: trim_option(&self.subcategory),
            group: trim_option(&self.group),
            technician_id: trim_option(&self.technician_id),
        }
    }
}

/// Input parameters for the close_request tool.
///
/// Request ID is required. Closure code and comments are optional.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CloseRequestInput {
    /// The unique ID of the ticket to close.
    pub request_id: String,

    /// Closure reason code (e.g., 'Success', 'Cancelled', 'Unable to Reproduce').
    #[serde(default)]
    pub closure_code: Option<String>,

    /// Explanation of how the issue was resolved or why it's being closed.
    #[serde(default)]
    pub closure_comments: Option<String>,
}

impl CloseRequestInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            request_id: self.request_id.trim().to_string(),
            closure_code: trim_option(&self.closure_code),
            closure_comments: trim_option(&self.closure_comments),
        }
    }
}

/// Input parameters for the add_note tool.
///
/// Request ID and content are required.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AddNoteInput {
    /// The unique ID of the ticket to add a note to.
    pub request_id: String,

    /// The note content (supports HTML formatting).
    pub content: String,

    /// If true, the note will be visible to the requester. Default: false (internal note).
    #[serde(default)]
    pub show_to_requester: Option<bool>,

    /// If true, send notification to assigned technician. Default: false.
    #[serde(default)]
    pub notify_technician: Option<bool>,
}

impl AddNoteInput {
    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            request_id: self.request_id.trim().to_string(),
            content: self.content.trim().to_string(),
            show_to_requester: self.show_to_requester,
            notify_technician: self.notify_technician,
        }
    }
}

/// Input parameters for the assign_request tool.
///
/// Request ID is required. At least one of technician_id or group must be provided.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AssignRequestInput {
    /// The unique ID of the ticket to assign.
    pub request_id: String,

    /// ID of the technician to assign (use list_technicians to find IDs).
    #[serde(default)]
    pub technician_id: Option<String>,

    /// Name of the support group to assign to.
    #[serde(default)]
    pub group: Option<String>,
}

impl AssignRequestInput {
    /// Returns true if at least one of technician_id or group is set.
    pub fn has_assignment(&self) -> bool {
        self.technician_id.is_some() || self.group.is_some()
    }

    /// Sanitizes input by trimming whitespace from all string fields.
    #[must_use]
    pub fn sanitize(self) -> Self {
        Self {
            request_id: self.request_id.trim().to_string(),
            technician_id: trim_option(&self.technician_id),
            group: trim_option(&self.group),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Sanitization tests
    // ========================================================================

    #[test]
    fn test_trim_option_trims_whitespace() {
        let s = Some("  hello  ".to_string());
        assert_eq!(trim_option(&s), Some("hello".to_string()));
    }

    #[test]
    fn test_trim_option_filters_empty() {
        let s = Some("   ".to_string());
        assert_eq!(trim_option(&s), None);
    }

    #[test]
    fn test_trim_option_none_stays_none() {
        let s: Option<String> = None;
        assert_eq!(trim_option(&s), None);
    }

    #[test]
    fn test_list_requests_input_sanitize() {
        let input = ListRequestsInput {
            status: Some("  Åben  ".to_string()),
            priority: Some("".to_string()),
            technician: Some("  Gorm Reventlow  ".to_string()),
            requester: None,
            open_only: Some(true),
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: None,
        };
        let sanitized = input.sanitize();
        assert_eq!(sanitized.status, Some("Åben".to_string()));
        assert_eq!(sanitized.priority, None); // Empty string becomes None
        assert_eq!(sanitized.technician, Some("Gorm Reventlow".to_string()));
        assert_eq!(sanitized.open_only, Some(true));
        assert_eq!(sanitized.limit, Some(10));
    }

    #[test]
    fn test_get_request_input_sanitize() {
        let input = GetRequestInput {
            request_id: "  12345  ".to_string(),
        };
        let sanitized = input.sanitize();
        assert_eq!(sanitized.request_id, "12345");
    }

    #[test]
    fn test_create_request_input_sanitize() {
        let input = CreateRequestInput {
            subject: "  Test subject  ".to_string(),
            description: Some("  Description  ".to_string()),
            requester_email: Some("  user@example.com  ".to_string()),
            priority: Some("   ".to_string()),
            category: None,
            subcategory: None,
            item: None,
            group: None,
            technician_id: None,
        };
        let sanitized = input.sanitize();
        assert_eq!(sanitized.subject, "Test subject");
        assert_eq!(sanitized.description, Some("Description".to_string()));
        assert_eq!(sanitized.requester_email, Some("user@example.com".to_string()));
        assert_eq!(sanitized.priority, None); // Whitespace-only becomes None
    }

    #[test]
    fn test_add_note_input_sanitize() {
        let input = AddNoteInput {
            request_id: "  123  ".to_string(),
            content: "  Note content  ".to_string(),
            show_to_requester: Some(true),
            notify_technician: None,
        };
        let sanitized = input.sanitize();
        assert_eq!(sanitized.request_id, "123");
        assert_eq!(sanitized.content, "Note content");
        assert_eq!(sanitized.show_to_requester, Some(true));
    }

    // ========================================================================
    // Deserialization tests
    // ========================================================================

    #[test]
    fn test_list_requests_input_deserialize_empty() {
        let json = "{}";
        let input: ListRequestsInput = serde_json::from_str(json).unwrap();
        assert!(input.status.is_none());
        assert!(input.priority.is_none());
    }

    #[test]
    fn test_list_requests_input_deserialize_with_filters() {
        let json = r#"{"status": "Open", "priority": "High", "limit": 10}"#;
        let input: ListRequestsInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.status.as_deref(), Some("Open"));
        assert_eq!(input.priority.as_deref(), Some("High"));
        assert_eq!(input.limit, Some(10));
    }

    #[test]
    fn test_get_request_input_deserialize() {
        let json = r#"{"request_id": "12345"}"#;
        let input: GetRequestInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.request_id, "12345");
    }

    #[test]
    fn test_list_technicians_input_deserialize() {
        let json = r#"{"group": "IT Support", "limit": 25}"#;
        let input: ListTechniciansInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.group.as_deref(), Some("IT Support"));
        assert_eq!(input.limit, Some(25));
    }

    // ========================================================================
    // Write operation input tests (M4)
    // ========================================================================

    #[test]
    fn test_create_request_input_minimal() {
        let json = r#"{"subject": "Test ticket"}"#;
        let input: CreateRequestInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.subject, "Test ticket");
        assert!(input.description.is_none());
        assert!(input.priority.is_none());
    }

    #[test]
    fn test_create_request_input_full() {
        let json = r#"{
            "subject": "Test ticket",
            "description": "Detailed description",
            "requester_email": "user@example.com",
            "priority": "High",
            "category": "Hardware",
            "subcategory": "Laptop",
            "group": "IT Support",
            "technician_id": "12345"
        }"#;
        let input: CreateRequestInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.subject, "Test ticket");
        assert_eq!(input.description.as_deref(), Some("Detailed description"));
        assert_eq!(input.requester_email.as_deref(), Some("user@example.com"));
        assert_eq!(input.priority.as_deref(), Some("High"));
        assert_eq!(input.technician_id.as_deref(), Some("12345"));
    }

    #[test]
    fn test_update_request_input_has_updates() {
        let json = r#"{"request_id": "123"}"#;
        let input: UpdateRequestInput = serde_json::from_str(json).unwrap();
        assert!(!input.has_updates());

        let json = r#"{"request_id": "123", "priority": "High"}"#;
        let input: UpdateRequestInput = serde_json::from_str(json).unwrap();
        assert!(input.has_updates());
    }

    #[test]
    fn test_close_request_input() {
        let json = r#"{
            "request_id": "123",
            "closure_code": "Success",
            "closure_comments": "Issue resolved"
        }"#;
        let input: CloseRequestInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.request_id, "123");
        assert_eq!(input.closure_code.as_deref(), Some("Success"));
        assert_eq!(input.closure_comments.as_deref(), Some("Issue resolved"));
    }

    #[test]
    fn test_add_note_input() {
        let json = r#"{
            "request_id": "123",
            "content": "This is a note",
            "show_to_requester": true
        }"#;
        let input: AddNoteInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.request_id, "123");
        assert_eq!(input.content, "This is a note");
        assert_eq!(input.show_to_requester, Some(true));
        assert!(input.notify_technician.is_none());
    }

    #[test]
    fn test_assign_request_input_has_assignment() {
        let json = r#"{"request_id": "123"}"#;
        let input: AssignRequestInput = serde_json::from_str(json).unwrap();
        assert!(!input.has_assignment());

        let json = r#"{"request_id": "123", "technician_id": "456"}"#;
        let input: AssignRequestInput = serde_json::from_str(json).unwrap();
        assert!(input.has_assignment());

        let json = r#"{"request_id": "123", "group": "IT Support"}"#;
        let input: AssignRequestInput = serde_json::from_str(json).unwrap();
        assert!(input.has_assignment());
    }
}
