//! Technician models for ServiceDesk Plus API.
//!
//! This module defines the data structures for SDP technicians,
//! who can be assigned to handle requests/tickets.

use serde::Deserialize;

/// A technician who can be assigned to handle requests.
///
/// Note: The SDP API returns many fields as nested objects.
/// We only capture the essential fields needed for display and assignment.
#[derive(Debug, Clone, Deserialize)]
pub struct Technician {
    /// Unique technician ID.
    pub id: String,

    /// Technician's display name.
    #[serde(default)]
    pub name: Option<String>,

    /// Technician's email address.
    #[serde(default)]
    pub email_id: Option<String>,

    /// First name.
    #[serde(default)]
    pub first_name: Option<String>,

    /// Last name.
    #[serde(default)]
    pub last_name: Option<String>,

    /// Phone number.
    #[serde(default)]
    pub phone: Option<String>,

    /// Mobile number.
    #[serde(default)]
    pub mobile: Option<String>,

    /// Job title (SDP returns this as "jobtitle").
    #[serde(default, alias = "jobtitle")]
    pub job_title: Option<String>,

    /// Department (can be a nested object with id/name).
    #[serde(default)]
    pub department: Option<serde_json::Value>,

    /// Whether the technician is currently active.
    #[serde(default)]
    pub is_active: Option<bool>,

    /// Associated site/location (can be a nested object).
    #[serde(default)]
    pub site: Option<serde_json::Value>,
}

impl Technician {
    /// Returns the display name, falling back to email or ID.
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.email_id.as_deref())
            .unwrap_or(&self.id)
    }

    /// Returns the email if present.
    pub fn email(&self) -> Option<&str> {
        self.email_id.as_deref()
    }
}

/// Response wrapper for list technicians operations.
#[derive(Debug, Clone, Deserialize)]
pub struct ListTechniciansResponse {
    /// List of technicians.
    #[serde(default)]
    pub technicians: Vec<Technician>,

    /// Pagination info (if requested).
    #[serde(default)]
    pub list_info: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_technician_display_name() {
        let tech = Technician {
            id: "123".to_string(),
            name: Some("John Doe".to_string()),
            email_id: Some("john@example.com".to_string()),
            first_name: None,
            last_name: None,
            phone: None,
            mobile: None,
            job_title: None,
            department: None,
            is_active: Some(true),
            site: None,
        };
        assert_eq!(tech.display_name(), "John Doe");
    }

    #[test]
    fn test_technician_display_name_fallback_to_email() {
        let tech = Technician {
            id: "123".to_string(),
            name: None,
            email_id: Some("john@example.com".to_string()),
            first_name: None,
            last_name: None,
            phone: None,
            mobile: None,
            job_title: None,
            department: None,
            is_active: None,
            site: None,
        };
        assert_eq!(tech.display_name(), "john@example.com");
    }

    #[test]
    fn test_technician_display_name_fallback_to_id() {
        let tech = Technician {
            id: "123".to_string(),
            name: None,
            email_id: None,
            first_name: None,
            last_name: None,
            phone: None,
            mobile: None,
            job_title: None,
            department: None,
            is_active: None,
            site: None,
        };
        assert_eq!(tech.display_name(), "123");
    }

    #[test]
    fn test_technician_deserialize() {
        let json = r#"{
            "id": "456",
            "name": "Jane Smith",
            "email_id": "jane@example.com",
            "is_active": true
        }"#;
        let tech: Technician = serde_json::from_str(json).unwrap();
        assert_eq!(tech.id, "456");
        assert_eq!(tech.name.as_deref(), Some("Jane Smith"));
        assert_eq!(tech.email(), Some("jane@example.com"));
        assert_eq!(tech.is_active, Some(true));
    }
}
