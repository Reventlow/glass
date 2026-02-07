//! Note models for ServiceDesk Plus API.
//!
//! This module defines the data structures for SDP request notes,
//! which are comments or updates added to tickets.

use serde::{Deserialize, Serialize};

use super::{deserialize_string_or_int, NamedEntity, SdpTimestamp};

/// A note attached to a request/ticket.
///
/// Notes can be internal (visible only to technicians) or
/// visible to the requester.
#[derive(Debug, Clone, Deserialize)]
pub struct Note {
    /// Unique note ID.
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub id: String,

    /// Note content (may contain HTML).
    /// SDP may use "description" or "content" for this field.
    #[serde(default, alias = "content")]
    pub description: Option<String>,

    /// Who created the note.
    /// SDP API uses "added_by" for notes.
    #[serde(default, alias = "added_by")]
    pub created_by: Option<NamedEntity>,

    /// When the note was created.
    /// SDP API uses "added_time" for notes.
    #[serde(default, alias = "added_time")]
    pub created_time: Option<SdpTimestamp>,

    /// Whether the note is visible to the requester.
    #[serde(default)]
    pub show_to_requester: Option<bool>,

    /// Whether to notify the assigned technician.
    #[serde(default)]
    pub notify_technician: Option<bool>,

    /// URL to fetch the note content.
    /// SDP sometimes returns content via this URL instead of inline.
    #[serde(default)]
    pub content_url: Option<String>,
}

impl Note {
    /// Returns the note content or a placeholder.
    pub fn display_content(&self) -> String {
        // Try description first (inline content or fetched content)
        if let Some(desc) = &self.description {
            return desc.clone();
        }
        // If content_url exists but we couldn't fetch, indicate that
        if self.content_url.is_some() {
            return "(Content could not be fetched)".to_string();
        }
        "(No content)".to_string()
    }

    /// Returns who created the note.
    pub fn display_created_by(&self) -> &str {
        self.created_by
            .as_ref()
            .and_then(|c| c.name.as_deref())
            .unwrap_or("Unknown")
    }
}

/// Request body for creating a new note.
///
/// Used when sending a POST request to add a note to a ticket.
#[derive(Debug, Clone, Serialize)]
pub struct CreateNoteRequest {
    /// The note content.
    pub description: String,

    /// Whether to show the note to the requester.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_to_requester: Option<bool>,

    /// Whether to notify the assigned technician.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_technician: Option<bool>,
}

impl CreateNoteRequest {
    /// Creates a new note request with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            description: content.into(),
            show_to_requester: None,
            notify_technician: None,
        }
    }

    /// Sets whether to show the note to the requester.
    pub fn with_show_to_requester(mut self, show: bool) -> Self {
        self.show_to_requester = Some(show);
        self
    }

    /// Sets whether to notify the assigned technician.
    pub fn with_notify_technician(mut self, notify: bool) -> Self {
        self.notify_technician = Some(notify);
        self
    }
}

/// Response wrapper for add note operations.
#[derive(Debug, Clone, Deserialize)]
pub struct AddNoteResponse {
    /// The created note.
    pub note: Note,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_display_content() {
        let note = Note {
            id: "123".to_string(),
            description: Some("Test note content".to_string()),
            created_by: None,
            created_time: None,
            show_to_requester: Some(false),
            notify_technician: None,
            content_url: None,
        };
        assert_eq!(note.display_content(), "Test note content");
    }

    #[test]
    fn test_note_display_content_empty() {
        let note = Note {
            id: "123".to_string(),
            description: None,
            created_by: None,
            created_time: None,
            show_to_requester: None,
            notify_technician: None,
            content_url: None,
        };
        assert_eq!(note.display_content(), "(No content)");
    }

    #[test]
    fn test_note_display_content_with_unfetched_url() {
        let note = Note {
            id: "123".to_string(),
            description: None,
            created_by: None,
            created_time: None,
            show_to_requester: None,
            notify_technician: None,
            content_url: Some("/api/v3/requests/123/notes/456".to_string()),
        };
        assert_eq!(note.display_content(), "(Content could not be fetched)");
    }

    #[test]
    fn test_create_note_request_builder() {
        let req = CreateNoteRequest::new("My note")
            .with_show_to_requester(true)
            .with_notify_technician(false);

        assert_eq!(req.description, "My note");
        assert_eq!(req.show_to_requester, Some(true));
        assert_eq!(req.notify_technician, Some(false));
    }

    #[test]
    fn test_create_note_request_serialization() {
        let req = CreateNoteRequest::new("Test content")
            .with_show_to_requester(false);

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["description"], "Test content");
        assert_eq!(json["show_to_requester"], false);
        assert!(json.get("notify_technician").is_none());
    }

    #[test]
    fn test_note_deserialize() {
        let json = r#"{
            "id": "456",
            "description": "Note content here",
            "show_to_requester": true,
            "notify_technician": false
        }"#;
        let note: Note = serde_json::from_str(json).unwrap();
        assert_eq!(note.id, "456");
        assert_eq!(note.description.as_deref(), Some("Note content here"));
        assert_eq!(note.show_to_requester, Some(true));
    }
}
