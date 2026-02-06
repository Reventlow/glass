//! MCP server implementation for Glass.
//!
//! This module defines the `GlassServer` struct that implements the MCP
//! `ServerHandler` trait, exposing ServiceDesk Plus operations as tools.

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};

use crate::models::{Note, Request, RequestSummary, Technician};
use crate::sdp_client::{ListParams, SdpClient};
use crate::tools::{
    AddNoteInput, AssignRequestInput, CloseRequestInput, CreateRequestInput, GetRequestInput,
    ListRequestsInput, ListTechniciansInput, UpdateRequestInput,
};

/// The Glass MCP server.
///
/// This server exposes ServiceDesk Plus operations as MCP tools.
#[derive(Clone)]
pub struct GlassServer {
    /// SDP client for API operations.
    sdp_client: SdpClient,
    /// Tool router for MCP tool dispatch.
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl GlassServer {
    /// Creates a new Glass server instance.
    ///
    /// # Arguments
    ///
    /// * `sdp_client` - The SDP client for API operations
    pub fn new(sdp_client: SdpClient) -> Self {
        Self {
            sdp_client,
            tool_router: Self::tool_router(),
        }
    }

    /// A simple ping tool to verify the server is running.
    ///
    /// This tool is useful for testing connectivity and validating
    /// that the MCP server is properly initialized.
    ///
    /// Returns "pong" on success.
    #[tool(description = "Test connectivity to the Glass MCP server. Returns 'pong' if the server is running correctly.")]
    fn ping(&self) -> String {
        tracing::debug!("ping tool called");
        "pong".to_string()
    }

    /// List service desk tickets with optional filters.
    ///
    /// Can filter by status, priority, technician, requester, or date range.
    /// Returns paginated results.
    #[tool(description = "List service desk tickets. Can filter by status, priority, technician, or date range. Returns paginated results with ticket ID, subject, status, and assignee.")]
    async fn list_requests(
        &self,
        Parameters(input): Parameters<ListRequestsInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(?input, "list_requests tool called");

        // Build ListParams from input
        let mut params = ListParams::new();

        // Apply filters
        if let Some(status) = input.status {
            params = params.with_status(status);
        }
        if let Some(priority) = input.priority {
            params = params.with_priority(priority);
        }
        if let Some(technician) = input.technician_id {
            params = params.with_technician(technician);
        }
        if let Some(requester) = input.requester_email {
            params = params.with_requester(requester);
        }

        // Apply pagination
        let limit = input.limit.unwrap_or(20).min(100);
        params = params.with_limit(limit);

        if let Some(offset) = input.offset {
            params = params.with_offset(offset);
        }

        // Note: created_after/created_before would need additional SDP-specific
        // date filtering logic that we can add in a future iteration

        // Execute the request
        let requests = self
            .sdp_client
            .list_requests(params)
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, "Failed to list requests");
                format!("Failed to list requests: {}", sanitized)
            })?;

        // Format the response
        Ok(format_request_list(&requests))
    }

    /// Get full details of a single service desk ticket.
    ///
    /// Returns complete information including description, notes, and history.
    #[tool(description = "Get full details of a single service desk ticket including description, notes, and history.")]
    async fn get_request(
        &self,
        Parameters(input): Parameters<GetRequestInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(request_id = %input.request_id, "get_request tool called");

        let request = self
            .sdp_client
            .get_request(&input.request_id)
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, request_id = %input.request_id, "Failed to get request");
                format!("Failed to get request {}: {}", input.request_id, sanitized)
            })?;

        // Format the response
        Ok(format_request_details(&request))
    }

    /// List technicians available for ticket assignment.
    ///
    /// Returns IDs and names so you can assign tickets to specific technicians.
    #[tool(description = "List all technicians available for ticket assignment. Returns IDs and names. Use the ID when assigning tickets.")]
    async fn list_technicians(
        &self,
        Parameters(input): Parameters<ListTechniciansInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(?input, "list_technicians tool called");

        let technicians = self
            .sdp_client
            .list_technicians(input.group.as_deref(), input.limit)
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, "Failed to list technicians");
                format!("Failed to list technicians: {}", sanitized)
            })?;

        // Format the response
        Ok(format_technician_list(&technicians))
    }

    // ========================================================================
    // Write tools (M4)
    // ========================================================================

    /// Create a new service desk ticket.
    ///
    /// Subject is required. Returns the created ticket with its assigned ID.
    #[tool(description = "Create a new service desk ticket. Subject is required. Returns the created ticket with its assigned ID.")]
    async fn create_request(
        &self,
        Parameters(input): Parameters<CreateRequestInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(subject = %input.subject, "create_request tool called");

        // Validate subject (already trimmed by sanitize)
        if input.subject.is_empty() {
            return Err("Subject is required and cannot be empty.".to_string());
        }
        if input.subject.len() > 250 {
            return Err(format!(
                "Subject exceeds maximum length of 250 characters (got {} characters).",
                input.subject.len()
            ));
        }

        let request = self
            .sdp_client
            .create_request(&input)
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, "Failed to create request");
                format!("Failed to create request: {}", sanitized)
            })?;

        Ok(format_create_result(&request))
    }

    /// Update an existing ticket's properties.
    ///
    /// Request ID is required. At least one field must be provided for update.
    #[tool(description = "Update an existing ticket's properties such as priority, status, category, or assignment. Request ID is required.")]
    async fn update_request(
        &self,
        Parameters(input): Parameters<UpdateRequestInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(request_id = %input.request_id, "update_request tool called");

        // Validate that at least one field is being updated
        if !input.has_updates() {
            return Err(
                "At least one field must be provided for update (subject, description, priority, status, category, subcategory, group, or technician_id).".to_string()
            );
        }

        // Validate subject length if provided (already trimmed by sanitize)
        if let Some(ref subject) = input.subject {
            if subject.is_empty() {
                return Err("Subject cannot be empty.".to_string());
            }
            if subject.len() > 250 {
                return Err(format!(
                    "Subject exceeds maximum length of 250 characters (got {} characters).",
                    subject.len()
                ));
            }
        }

        let request = self
            .sdp_client
            .update_request(&input.request_id, &input)
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, request_id = %input.request_id, "Failed to update request");
                format!("Failed to update request {}: {}", input.request_id, sanitized)
            })?;

        Ok(format_update_result(&request))
    }

    /// Close a ticket with closure reason and comments.
    ///
    /// Request ID is required. Closure code and comments are optional.
    #[tool(description = "Close a ticket with closure reason and comments. Request ID is required.")]
    async fn close_request(
        &self,
        Parameters(input): Parameters<CloseRequestInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(request_id = %input.request_id, "close_request tool called");

        let request = self
            .sdp_client
            .close_request(
                &input.request_id,
                input.closure_code.as_deref(),
                input.closure_comments.as_deref(),
            )
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, request_id = %input.request_id, "Failed to close request");
                format!("Failed to close request {}: {}", input.request_id, sanitized)
            })?;

        Ok(format_close_result(&request))
    }

    /// Add a note to a ticket.
    ///
    /// Notes can be internal or visible to requester.
    #[tool(description = "Add a note to a ticket. Notes can be internal (technicians only) or visible to the requester. Request ID and content are required.")]
    async fn add_note(
        &self,
        Parameters(input): Parameters<AddNoteInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(request_id = %input.request_id, "add_note tool called");

        // Validate content (already trimmed by sanitize)
        if input.content.is_empty() {
            return Err("Note content is required and cannot be empty.".to_string());
        }

        let note = self
            .sdp_client
            .add_note(
                &input.request_id,
                &input.content,
                input.show_to_requester,
                input.notify_technician,
            )
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, request_id = %input.request_id, "Failed to add note");
                format!("Failed to add note to request {}: {}", input.request_id, sanitized)
            })?;

        Ok(format_add_note_result(&input.request_id, &note))
    }

    /// Assign a ticket to a technician or support group.
    ///
    /// At least one of technician_id or group must be provided.
    #[tool(description = "Assign a ticket to a technician or support group. At least one of technician_id or group must be provided.")]
    async fn assign_request(
        &self,
        Parameters(input): Parameters<AssignRequestInput>,
    ) -> Result<String, String> {
        // Sanitize input
        let input = input.sanitize();
        tracing::debug!(request_id = %input.request_id, "assign_request tool called");

        // Validate that at least one assignment target is provided
        if !input.has_assignment() {
            return Err(
                "At least one of technician_id or group must be provided for assignment."
                    .to_string(),
            );
        }

        let request = self
            .sdp_client
            .assign_request(
                &input.request_id,
                input.technician_id.as_deref(),
                input.group.as_deref(),
            )
            .await
            .map_err(|e| {
                let sanitized = self.sanitize_error(&e);
                tracing::error!(error = %sanitized, request_id = %input.request_id, "Failed to assign request");
                format!("Failed to assign request {}: {}", input.request_id, sanitized)
            })?;

        Ok(format_assign_result(&request, &input))
    }

    /// Sanitizes an error message to remove any API key.
    fn sanitize_error(&self, error: &crate::error::GlassError) -> String {
        error.sanitized_display(self.sdp_client.api_key_for_sanitization())
    }
}

#[tool_handler]
impl ServerHandler for GlassServer {
    /// Returns server information for the MCP initialize handshake.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Glass provides access to ServiceDesk Plus tickets. \
                 Use list_requests to find tickets, get_request for details, \
                 and list_technicians to see available assignees. \
                 Create tickets with create_request, modify with update_request, \
                 close with close_request, add notes with add_note, and \
                 assign with assign_request. Start with 'ping' to verify connectivity."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ============================================================================
// Response formatting helpers
// ============================================================================

/// Maximum length for description fields before truncation.
const MAX_DESCRIPTION_LENGTH: usize = 2000;

/// Truncates a string if it exceeds the maximum length.
///
/// If truncated, appends "... [truncated]" to indicate the content was cut.
fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        // Find a good break point (word boundary if possible)
        let mut end = max_length - 15; // Leave room for "... [truncated]"
        if let Some(space_pos) = text[..end].rfind(char::is_whitespace) {
            end = space_pos;
        }
        format!("{}... [truncated]", &text[..end])
    }
}

/// Formats a list of request summaries as human-readable text.
fn format_request_list(requests: &[RequestSummary]) -> String {
    if requests.is_empty() {
        return "No tickets found matching the criteria.".to_string();
    }

    let mut output = format!("Found {} ticket(s):\n\n", requests.len());

    for req in requests {
        output.push_str(&format!("#{} - {}\n", req.id, req.display_subject()));
        output.push_str(&format!(
            "   Status: {} | Priority: {} | Assignee: {}\n",
            req.display_status(),
            req.display_priority(),
            req.display_technician()
        ));
        output.push_str(&format!("   Requester: {}\n", req.display_requester()));

        if let Some(created) = req.created_time.as_ref().and_then(|t| t.display()) {
            output.push_str(&format!("   Created: {}\n", created));
        }

        output.push('\n');
    }

    output
}

/// Formats full request details as human-readable text.
fn format_request_details(request: &Request) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "Ticket #{}: {}\n",
        request.id,
        request.display_subject()
    ));
    output.push_str(&"=".repeat(60));
    output.push('\n');

    // Status information
    output.push_str(&format!("\nStatus: {}\n", request.display_status()));
    output.push_str(&format!("Priority: {}\n", request.display_priority()));

    if let Some(urgency) = request.urgency.as_ref().and_then(|u| u.name.as_deref()) {
        output.push_str(&format!("Urgency: {}\n", urgency));
    }
    if let Some(impact) = request.impact.as_ref().and_then(|i| i.name.as_deref()) {
        output.push_str(&format!("Impact: {}\n", impact));
    }

    // Category path
    let category_path = request.category_path();
    if category_path != "Uncategorized" {
        output.push_str(&format!("Category: {}\n", category_path));
    }

    // People
    output.push_str(&format!("\nRequester: {}\n", request.display_requester()));
    output.push_str(&format!("Assigned to: {}\n", request.display_technician()));

    if let Some(group) = request.display_group() {
        output.push_str(&format!("Group: {}\n", group));
    }

    // Timestamps
    output.push_str("\n--- Timestamps ---\n");
    if let Some(created) = request.created_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("Created: {}\n", created));
    }
    if let Some(updated) = request.last_updated_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("Last Updated: {}\n", updated));
    }
    if let Some(due) = request.due_by_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("Due By: {}\n", due));
    }

    // Flags
    if request.is_overdue == Some(true) {
        output.push_str("\n[OVERDUE]\n");
    }

    // Description (truncated if too long)
    if let Some(description) = &request.description {
        output.push_str("\n--- Description ---\n");
        output.push_str(&truncate_text(description, MAX_DESCRIPTION_LENGTH));
        output.push('\n');
    }

    // Resolution (if present, truncated if too long)
    if let Some(resolution) = &request.resolution {
        if let Some(content) = &resolution.content {
            output.push_str("\n--- Resolution ---\n");
            output.push_str(&truncate_text(content, MAX_DESCRIPTION_LENGTH));
            output.push('\n');

            if let Some(submitted_by) = resolution
                .submitted_by
                .as_ref()
                .and_then(|s| s.name.as_deref())
            {
                output.push_str(&format!("Submitted by: {}\n", submitted_by));
            }
            if let Some(submitted_on) = resolution.submitted_on.as_ref().and_then(|t| t.display()) {
                output.push_str(&format!("Submitted on: {}\n", submitted_on));
            }
        }
    }

    // Closure info (if present)
    if let Some(closure) = &request.closure_info {
        output.push_str("\n--- Closure Info ---\n");
        if let Some(code) = closure
            .closure_code
            .as_ref()
            .and_then(|c| c.name.as_deref())
        {
            output.push_str(&format!("Closure Code: {}\n", code));
        }
        if let Some(comments) = &closure.closure_comments {
            output.push_str(&format!("Comments: {}\n", comments));
        }
        if let Some(closed_by) = closure.closed_by.as_ref().and_then(|c| c.name.as_deref()) {
            output.push_str(&format!("Closed by: {}\n", closed_by));
        }
        if let Some(closed_time) = closure.closed_time.as_ref().and_then(|t| t.display()) {
            output.push_str(&format!("Closed at: {}\n", closed_time));
        }
    }

    output
}

/// Formats a list of technicians as human-readable text.
fn format_technician_list(technicians: &[Technician]) -> String {
    if technicians.is_empty() {
        return "No technicians found.".to_string();
    }

    let mut output = format!("Found {} technician(s):\n\n", technicians.len());

    for tech in technicians {
        output.push_str(&format!("ID: {} | Name: {}", tech.id, tech.display_name()));

        if let Some(email) = tech.email() {
            output.push_str(&format!(" | Email: {}", email));
        }

        if let Some(active) = tech.is_active {
            if !active {
                output.push_str(" [INACTIVE]");
            }
        }

        output.push('\n');
    }

    output
}

// ============================================================================
// Write operation formatting helpers (M4)
// ============================================================================

/// Formats the result of a create request operation.
fn format_create_result(request: &Request) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Successfully created ticket #{}: {}\n\n",
        request.id,
        request.display_subject()
    ));

    output.push_str(&format!("Status: {}\n", request.display_status()));
    output.push_str(&format!("Priority: {}\n", request.display_priority()));
    output.push_str(&format!("Assigned to: {}\n", request.display_technician()));

    if let Some(group) = request.display_group() {
        output.push_str(&format!("Group: {}\n", group));
    }

    output.push_str(&format!("\nRequester: {}\n", request.display_requester()));

    if let Some(created) = request.created_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("Created: {}\n", created));
    }

    output.push_str("\nNext steps:\n");
    output.push_str(&format!(
        "  - View details: use get_request with request_id=\"{}\"\n",
        request.id
    ));
    output.push_str(&format!(
        "  - Add notes: use add_note with request_id=\"{}\"\n",
        request.id
    ));

    output
}

/// Formats the result of an update request operation.
fn format_update_result(request: &Request) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Successfully updated ticket #{}: {}\n\n",
        request.id,
        request.display_subject()
    ));

    output.push_str("Current state:\n");
    output.push_str(&format!("  Status: {}\n", request.display_status()));
    output.push_str(&format!("  Priority: {}\n", request.display_priority()));
    output.push_str(&format!("  Assigned to: {}\n", request.display_technician()));

    if let Some(group) = request.display_group() {
        output.push_str(&format!("  Group: {}\n", group));
    }

    let category_path = request.category_path();
    if category_path != "Uncategorized" {
        output.push_str(&format!("  Category: {}\n", category_path));
    }

    if let Some(updated) = request.last_updated_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("\nLast updated: {}\n", updated));
    }

    output
}

/// Formats the result of a close request operation.
fn format_close_result(request: &Request) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Successfully closed ticket #{}: {}\n\n",
        request.id,
        request.display_subject()
    ));

    output.push_str(&format!("Status: {}\n", request.display_status()));

    if let Some(closure) = &request.closure_info {
        if let Some(code) = closure
            .closure_code
            .as_ref()
            .and_then(|c| c.name.as_deref())
        {
            output.push_str(&format!("Closure Code: {}\n", code));
        }
        if let Some(comments) = &closure.closure_comments {
            output.push_str(&format!("Closure Comments: {}\n", comments));
        }
        if let Some(closed_time) = closure.closed_time.as_ref().and_then(|t| t.display()) {
            output.push_str(&format!("Closed at: {}\n", closed_time));
        }
    }

    output
}

/// Formats the result of an add note operation.
fn format_add_note_result(request_id: &str, note: &Note) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Successfully added note #{} to ticket #{}.\n\n",
        note.id, request_id
    ));

    let visibility = if note.show_to_requester == Some(true) {
        "Visible to requester"
    } else {
        "Internal (technicians only)"
    };
    output.push_str(&format!("Visibility: {}\n", visibility));

    if let Some(created) = note.created_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("Created: {}\n", created));
    }

    if note.notify_technician == Some(true) {
        output.push_str("Technician notification: Sent\n");
    }

    output
}

/// Formats the result of an assign request operation.
fn format_assign_result(request: &Request, input: &AssignRequestInput) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Successfully assigned ticket #{}: {}\n\n",
        request.id,
        request.display_subject()
    ));

    if input.technician_id.is_some() {
        output.push_str(&format!(
            "Technician: {}\n",
            request.display_technician()
        ));
    }

    if input.group.is_some() {
        if let Some(group) = request.display_group() {
            output.push_str(&format!("Group: {}\n", group));
        }
    }

    if let Some(updated) = request.last_updated_time.as_ref().and_then(|t| t.display()) {
        output.push_str(&format!("\nUpdated: {}\n", updated));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::{NamedEntity, SdpTimestamp};

    // ========================================================================
    // Truncation tests
    // ========================================================================

    #[test]
    fn test_truncate_text_short_text() {
        let text = "Short text";
        let result = truncate_text(text, 100);
        assert_eq!(result, text);
    }

    #[test]
    fn test_truncate_text_exact_length() {
        let text = "x".repeat(100);
        let result = truncate_text(&text, 100);
        assert_eq!(result, text);
    }

    #[test]
    fn test_truncate_text_long_text() {
        let text = "word ".repeat(500); // 2500 chars
        let result = truncate_text(&text, 100);
        assert!(result.len() <= 100);
        assert!(result.ends_with("... [truncated]"));
    }

    #[test]
    fn test_truncate_text_breaks_at_word() {
        let text = "hello world this is a long sentence that needs truncation";
        let result = truncate_text(text, 30);
        assert!(result.ends_with("... [truncated]"));
        // Should break at a word boundary, not in the middle of a word
        assert!(!result.contains("sente... [truncated]"));
    }

    fn test_config() -> Config {
        Config {
            base_url: "https://test.example.com".to_string(),
            api_key: "test_key_12345".to_string(),
        }
    }

    fn test_client() -> SdpClient {
        SdpClient::new(&test_config()).expect("Failed to create test client")
    }

    #[test]
    fn test_server_creation() {
        let client = test_client();
        let server = GlassServer::new(client);
        let info = server.get_info();
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_server_info_has_tools_capability() {
        let client = test_client();
        let server = GlassServer::new(client);
        let info = server.get_info();
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_ping_tool_returns_pong() {
        let client = test_client();
        let server = GlassServer::new(client);
        let result = server.ping();
        assert_eq!(result, "pong");
    }

    #[test]
    fn test_format_request_list_empty() {
        let result = format_request_list(&[]);
        assert_eq!(result, "No tickets found matching the criteria.");
    }

    #[test]
    fn test_format_request_list_with_items() {
        let requests = vec![RequestSummary {
            id: "123".to_string(),
            subject: Some("Test ticket".to_string()),
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
            created_time: Some(SdpTimestamp {
                value: None,
                display_value: Some("Feb 6, 2026".to_string()),
            }),
            last_updated_time: None,
            due_by_time: None,
            request_type: None,
            category: None,
            subcategory: None,
            site: None,
            group: None,
        }];

        let result = format_request_list(&requests);
        assert!(result.contains("#123"));
        assert!(result.contains("Test ticket"));
        assert!(result.contains("Open"));
        assert!(result.contains("High"));
        assert!(result.contains("John Doe"));
    }

    #[test]
    fn test_format_technician_list_empty() {
        let result = format_technician_list(&[]);
        assert_eq!(result, "No technicians found.");
    }

    #[test]
    fn test_format_technician_list_with_items() {
        let technicians = vec![Technician {
            id: "456".to_string(),
            name: Some("Jane Smith".to_string()),
            email_id: Some("jane@example.com".to_string()),
            first_name: None,
            last_name: None,
            phone: None,
            mobile: None,
            job_title: None,
            department: None,
            is_active: Some(true),
            site: None,
        }];

        let result = format_technician_list(&technicians);
        assert!(result.contains("456"));
        assert!(result.contains("Jane Smith"));
        assert!(result.contains("jane@example.com"));
    }

    // ========================================================================
    // Write operation formatting tests (M4)
    // ========================================================================

    fn create_test_request() -> Request {
        Request {
            id: "123".to_string(),
            subject: Some("Test ticket".to_string()),
            description: Some("Test description".to_string()),
            status: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("Open".to_string()),
            }),
            priority: Some(NamedEntity {
                id: Some("2".to_string()),
                name: Some("High".to_string()),
            }),
            urgency: None,
            impact: None,
            technician: Some(NamedEntity {
                id: Some("456".to_string()),
                name: Some("John Doe".to_string()),
            }),
            requester: Some(NamedEntity {
                id: Some("789".to_string()),
                name: Some("Jane User".to_string()),
            }),
            request_type: None,
            category: None,
            subcategory: None,
            item: None,
            site: None,
            group: Some(NamedEntity {
                id: Some("10".to_string()),
                name: Some("IT Support".to_string()),
            }),
            level: None,
            mode: None,
            service: None,
            created_time: Some(SdpTimestamp {
                value: None,
                display_value: Some("Feb 6, 2026".to_string()),
            }),
            last_updated_time: Some(SdpTimestamp {
                value: None,
                display_value: Some("Feb 6, 2026 10:30 AM".to_string()),
            }),
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
        }
    }

    #[test]
    fn test_format_create_result() {
        let request = create_test_request();
        let result = format_create_result(&request);

        assert!(result.contains("Successfully created ticket #123"));
        assert!(result.contains("Test ticket"));
        assert!(result.contains("Status: Open"));
        assert!(result.contains("Priority: High"));
        assert!(result.contains("John Doe"));
        assert!(result.contains("IT Support"));
        assert!(result.contains("Next steps:"));
    }

    #[test]
    fn test_format_update_result() {
        let request = create_test_request();
        let result = format_update_result(&request);

        assert!(result.contains("Successfully updated ticket #123"));
        assert!(result.contains("Current state:"));
        assert!(result.contains("Status: Open"));
        assert!(result.contains("Priority: High"));
    }

    #[test]
    fn test_format_close_result() {
        let mut request = create_test_request();
        request.status = Some(NamedEntity {
            id: Some("5".to_string()),
            name: Some("Closed".to_string()),
        });
        request.closure_info = Some(crate::models::ClosureInfo {
            closure_code: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("Success".to_string()),
            }),
            closure_comments: Some("Issue resolved".to_string()),
            closed_by: None,
            closed_time: Some(SdpTimestamp {
                value: None,
                display_value: Some("Feb 6, 2026 11:00 AM".to_string()),
            }),
        });

        let result = format_close_result(&request);

        assert!(result.contains("Successfully closed ticket #123"));
        assert!(result.contains("Status: Closed"));
        assert!(result.contains("Closure Code: Success"));
        assert!(result.contains("Issue resolved"));
    }

    #[test]
    fn test_format_add_note_result() {
        use crate::models::Note;

        let note = Note {
            id: "999".to_string(),
            description: Some("Test note content".to_string()),
            created_by: None,
            created_time: Some(SdpTimestamp {
                value: None,
                display_value: Some("Feb 6, 2026 10:45 AM".to_string()),
            }),
            show_to_requester: Some(false),
            notify_technician: Some(true),
        };

        let result = format_add_note_result("123", &note);

        assert!(result.contains("Successfully added note #999 to ticket #123"));
        assert!(result.contains("Internal (technicians only)"));
        assert!(result.contains("Technician notification: Sent"));
    }

    #[test]
    fn test_format_assign_result() {
        use crate::tools::AssignRequestInput;

        let request = create_test_request();
        let input = AssignRequestInput {
            request_id: "123".to_string(),
            technician_id: Some("456".to_string()),
            group: Some("IT Support".to_string()),
        };

        let result = format_assign_result(&request, &input);

        assert!(result.contains("Successfully assigned ticket #123"));
        assert!(result.contains("Technician: John Doe"));
        assert!(result.contains("Group: IT Support"));
    }
}
