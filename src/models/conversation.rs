//! Conversation models for ServiceDesk Plus API.
//!
//! This module defines the data structures for SDP request conversations,
//! which are email replies and messages exchanged with requesters.

use serde::Deserialize;

use super::{deserialize_string_or_int, NamedEntity, SdpTimestamp};

/// A conversation entry attached to a request/ticket.
///
/// Conversations represent email exchanges between technicians and requesters.
#[derive(Debug, Clone, Deserialize)]
pub struct Conversation {
    /// Unique conversation ID.
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub id: String,

    /// Conversation content (may contain HTML).
    /// SDP may use "description", "content", or "body" for this field.
    #[serde(default, alias = "content", alias = "body")]
    pub description: Option<String>,

    /// Who sent this message.
    #[serde(default, rename = "from")]
    pub from_user: Option<NamedEntity>,

    /// Recipients of the message.
    #[serde(default)]
    pub to: Option<Vec<String>>,

    /// When the conversation was created/sent.
    /// SDP uses "sent_time" for conversations.
    #[serde(default, alias = "created_time", alias = "created_date")]
    pub sent_time: Option<SdpTimestamp>,

    /// Type of conversation (e.g., "Reply", "Forward", "conversation").
    /// SDP returns this as a plain string, not a NamedEntity.
    #[serde(default, rename = "type")]
    pub conversation_type: Option<String>,

    /// Whether this is an incoming or outgoing message.
    #[serde(default)]
    pub is_incoming: Option<bool>,

    /// Subject of the conversation (for email threads).
    #[serde(default)]
    pub subject: Option<String>,

    /// URL to fetch the conversation content.
    /// SDP returns content via this URL instead of inline.
    #[serde(default)]
    pub content_url: Option<String>,

    /// Whether the conversation has attachments.
    #[serde(default)]
    pub has_attachments: Option<bool>,

    /// Whether to show to requester.
    #[serde(default)]
    pub show_to_requester: Option<bool>,
}

impl Conversation {
    /// Returns the conversation content or a placeholder.
    pub fn display_content(&self) -> String {
        // Try description first (inline content or fetched content)
        if let Some(desc) = &self.description {
            return desc.clone();
        }
        // If content_url exists but we couldn't fetch, indicate that
        if self.content_url.is_some() {
            return "(Content could not be fetched)".to_string();
        }
        // Try subject as fallback
        if let Some(subj) = &self.subject {
            return format!("[Subject: {}]", subj);
        }
        "(No content)".to_string()
    }

    /// Returns the timestamp for display.
    pub fn display_time(&self) -> Option<&str> {
        self.sent_time.as_ref().and_then(|t| t.display())
    }

    /// Returns who sent the message.
    pub fn display_from(&self) -> &str {
        self.from_user
            .as_ref()
            .and_then(|f| f.name.as_deref())
            .unwrap_or("Unknown")
    }

    /// Returns the direction indicator.
    pub fn direction(&self) -> &str {
        match self.is_incoming {
            Some(true) => "Incoming",
            Some(false) => "Outgoing",
            None => "Unknown",
        }
    }
}

/// Response wrapper for list conversations operations.
#[derive(Debug, Clone, Deserialize)]
pub struct ListConversationsResponse {
    /// List of conversations.
    #[serde(default)]
    pub conversations: Vec<Conversation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_display_content() {
        let conv = Conversation {
            id: "123".to_string(),
            description: Some("Test message content".to_string()),
            from_user: None,
            to: None,
            sent_time: None,
            conversation_type: None,
            is_incoming: Some(true),
            subject: None,
            content_url: None,
            has_attachments: None,
            show_to_requester: None,
        };
        assert_eq!(conv.display_content(), "Test message content");
    }

    #[test]
    fn test_conversation_display_content_empty() {
        let conv = Conversation {
            id: "123".to_string(),
            description: None,
            from_user: None,
            to: None,
            sent_time: None,
            conversation_type: None,
            is_incoming: None,
            subject: None,
            content_url: None,
            has_attachments: None,
            show_to_requester: None,
        };
        assert_eq!(conv.display_content(), "(No content)");
    }

    #[test]
    fn test_conversation_direction() {
        let incoming = Conversation {
            id: "1".to_string(),
            description: None,
            from_user: None,
            to: None,
            sent_time: None,
            conversation_type: None,
            is_incoming: Some(true),
            subject: None,
            content_url: None,
            has_attachments: None,
            show_to_requester: None,
        };
        assert_eq!(incoming.direction(), "Incoming");

        let outgoing = Conversation {
            id: "2".to_string(),
            description: None,
            from_user: None,
            to: None,
            sent_time: None,
            conversation_type: None,
            is_incoming: Some(false),
            subject: None,
            content_url: None,
            has_attachments: None,
            show_to_requester: None,
        };
        assert_eq!(outgoing.direction(), "Outgoing");
    }

    #[test]
    fn test_conversation_deserialize() {
        let json = r#"{
            "id": "456",
            "description": "Hello, this is a reply",
            "is_incoming": true
        }"#;
        let conv: Conversation = serde_json::from_str(json).unwrap();
        assert_eq!(conv.id, "456");
        assert_eq!(conv.description.as_deref(), Some("Hello, this is a reply"));
        assert_eq!(conv.is_incoming, Some(true));
    }
}
