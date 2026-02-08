//! HTTP client for ServiceDesk Plus API.
//!
//! This module provides the `SdpClient` struct for making authenticated
//! requests to the ServiceDesk Plus REST API.
//!
//! # Retry Logic
//!
//! The client automatically retries transient failures:
//! - HTTP 429 (rate limit): Exponential backoff starting at 100ms
//! - HTTP 502/503/504: Single retry after 500ms
//! - Timeouts: Single retry
//!
//! Client errors (4xx except 429) are not retried.
//!
//! # Security
//!
//! The API key is never logged. All error messages are sanitized before logging.

use std::future::Future;
use std::time::Duration;

use reqwest::{Client, Method, StatusCode};
use url::Url;

use crate::config::Config;
use crate::error::GlassError;
use crate::models::{
    AddNoteResponse, Conversation, CreateNoteRequest, GetRequestResponse,
    ListConversationsResponse, ListInfo, ListNotesResponse, ListRequestsResponse,
    ListTechniciansResponse, Note, Request, RequestSummary, SdpResponse, SearchCriteria,
    Technician,
};
use crate::tools::{CreateRequestInput, UpdateRequestInput};

/// Default request timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// The Accept header value for SDP API v3.
const SDP_ACCEPT_HEADER: &str = "application/vnd.manageengine.sdp.v3+json";

/// Maximum number of retry attempts for transient failures.
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Initial delay for exponential backoff (milliseconds).
const INITIAL_BACKOFF_MS: u64 = 100;

/// Delay before retrying after server error (milliseconds).
const SERVER_ERROR_DELAY_MS: u64 = 500;

/// Maximum length for HTTP error response bodies to avoid leaking verbose SDP internals.
const MAX_ERROR_BODY_LEN: usize = 500;

/// HTTP client for ServiceDesk Plus API.
///
/// Handles authentication, request formatting, and response parsing
/// for all SDP API operations.
///
/// # Example
///
/// ```ignore
/// let config = Config::from_env()?;
/// let client = SdpClient::new(&config)?;
///
/// let requests = client.list_requests(ListParams::default()).await?;
/// ```
#[derive(Clone)]
pub struct SdpClient {
    /// The underlying HTTP client (cloning is cheap).
    http: Client,

    /// Base URL for the SDP API (e.g., `https://servicedesk.example.com/api/v3`).
    base_url: String,

    /// API key for authentication.
    /// SECURITY: Never log this value!
    api_key: String,
}

impl SdpClient {
    /// Creates a new SDP client from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing base URL and API key
    ///
    /// # Errors
    ///
    /// Returns `GlassError::HttpClient` if the HTTP client fails to initialize.
    pub fn new(config: &Config) -> Result<Self, GlassError> {
        let http = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(GlassError::HttpClient)?;

        // Ensure base_url ends with /api/v3
        let base_url = Self::normalize_base_url(&config.base_url);

        Ok(Self {
            http,
            base_url,
            api_key: config.api_key().to_string(),
        })
    }

    /// Normalizes the base URL to ensure it includes the API path.
    fn normalize_base_url(url: &str) -> String {
        let url = url.trim_end_matches('/');
        if url.ends_with("/api/v3") {
            url.to_string()
        } else if url.ends_with("/api") {
            format!("{}/v3", url)
        } else {
            format!("{}/api/v3", url)
        }
    }

    /// Returns a reference to the API key for sanitization purposes.
    ///
    /// This should ONLY be used for sanitizing error messages, never for logging.
    pub(crate) fn api_key_for_sanitization(&self) -> &str {
        &self.api_key
    }

    /// Validates that an ID is a numeric string, as expected by the SDP API.
    ///
    /// SDP uses strictly numeric IDs for all entities. This prevents
    /// path traversal or injection via malformed IDs interpolated into URLs.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID string to validate
    /// * `field_name` - Name of the field for error messages (e.g., "request_id")
    ///
    /// # Errors
    ///
    /// Returns `GlassError::Validation` if the ID is empty or contains non-digit characters.
    fn validate_id(id: &str, field_name: &str) -> Result<(), GlassError> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(GlassError::validation(format!(
                "{} must be a numeric string, got: {:?}",
                field_name,
                id.chars().take(50).collect::<String>()
            )));
        }
        Ok(())
    }

    /// Returns the web URL for viewing a request in the ServiceDesk Plus UI.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The unique request ID
    ///
    /// # Returns
    ///
    /// A URL string that can be used to view the request in a browser.
    pub fn request_web_url(&self, request_id: &str) -> String {
        // Remove /api/v3 suffix to get the base web URL
        let web_base = self
            .base_url
            .trim_end_matches("/api/v3")
            .trim_end_matches("/api");
        format!(
            "{}/WorkOrder.do?woMode=viewWO&woID={}",
            web_base,
            urlencoding::encode(request_id)
        )
    }

    /// Tests connectivity to the SDP server.
    ///
    /// Makes a simple API call to verify the server is reachable and
    /// authentication is working.
    ///
    /// # Errors
    ///
    /// Returns `GlassError::ConnectionTest` if the connection fails,
    /// with details about the failure reason.
    pub async fn test_connection(&self) -> Result<(), GlassError> {
        tracing::debug!("Testing connection to SDP server");

        // Try to list a single request as a connectivity test
        let result = self.list_requests(ListParams::new().with_limit(1)).await;

        match result {
            Ok(_) => {
                tracing::info!("Connection test successful");
                Ok(())
            }
            Err(GlassError::Authentication) => {
                Err(GlassError::connection_test(
                    "Authentication failed - verify SDP_API_KEY is correct",
                ))
            }
            Err(GlassError::Timeout { duration, .. }) => {
                Err(GlassError::connection_test(format!(
                    "Connection timed out after {:?} - verify SDP_BASE_URL is correct and server is reachable",
                    duration
                )))
            }
            Err(GlassError::Http(e)) => {
                let message = GlassError::sanitize_message(&e.to_string(), &self.api_key);
                Err(GlassError::connection_test(format!(
                    "HTTP error: {} - verify SDP_BASE_URL is correct",
                    message
                )))
            }
            Err(e) => {
                let message = GlassError::sanitize_message(&e.to_string(), &self.api_key);
                Err(GlassError::connection_test(message))
            }
        }
    }

    /// Executes an operation with retry logic for transient failures.
    ///
    /// Retries on:
    /// - HTTP 429 (rate limit) with exponential backoff
    /// - HTTP 502/503/504 with fixed delay
    /// - Timeouts with fixed delay
    ///
    /// Does not retry on client errors (4xx except 429).
    async fn with_retry<T, F, Fut>(&self, operation: &str, f: F) -> Result<T, GlassError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, GlassError>>,
    {
        let mut delay = Duration::from_millis(INITIAL_BACKOFF_MS);
        let mut attempts = 0u32;

        loop {
            attempts += 1;
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if e.is_retryable() && attempts < MAX_RETRY_ATTEMPTS => {
                    // Determine delay based on error type
                    let actual_delay = if e.is_rate_limit() {
                        // Use provided retry_after or exponential backoff
                        e.retry_after().unwrap_or(delay)
                    } else if matches!(e, GlassError::ServiceUnavailable { .. }) {
                        // Fixed delay for server errors
                        Duration::from_millis(SERVER_ERROR_DELAY_MS)
                    } else {
                        delay
                    };

                    tracing::debug!(
                        operation = operation,
                        attempt = attempts,
                        max_attempts = MAX_RETRY_ATTEMPTS,
                        delay_ms = actual_delay.as_millis() as u64,
                        error = %GlassError::sanitize_message(&e.to_string(), &self.api_key),
                        "Retrying after transient error"
                    );

                    tokio::time::sleep(actual_delay).await;

                    // Exponential backoff for next attempt (if rate limited)
                    if e.is_rate_limit() {
                        delay *= 2;
                    }
                }
                Err(e) => {
                    // Log the final error (sanitized)
                    if attempts > 1 {
                        tracing::debug!(
                            operation = operation,
                            attempts = attempts,
                            "All retry attempts exhausted"
                        );
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Makes a GET request to the SDP API.
    ///
    /// # Arguments
    ///
    /// * `path` - API endpoint path (e.g., "/requests")
    /// * `input_data` - Optional input data to send as query parameter
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected response data type
    async fn get<T>(
        &self,
        path: &str,
        input_data: Option<serde_json::Value>,
    ) -> Result<T, GlassError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.request::<T>(Method::GET, path, input_data).await
    }

    /// Makes a request to the SDP API.
    ///
    /// Handles authentication, input data formatting, and response parsing.
    /// This is the low-level request method without retry logic.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method
    /// * `path` - API endpoint path
    /// * `input_data` - Optional input data (sent as form parameter for POST/PUT,
    ///   query parameter for GET)
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected response data type
    async fn request_inner<T>(
        &self,
        method: Method,
        path: &str,
        input_data: Option<serde_json::Value>,
    ) -> Result<T, GlassError>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);

        tracing::debug!(
            method = %method,
            path = %path,
            "Making SDP API request"
        );

        let mut req = self
            .http
            .request(method.clone(), &url)
            .header("authtoken", &self.api_key)
            .header("Accept", SDP_ACCEPT_HEADER);

        // Add input_data based on HTTP method
        if let Some(data) = input_data {
            let input_json = serde_json::to_string(&data).map_err(GlassError::Serialization)?;

            match method {
                Method::GET => {
                    // For GET, send as query parameter
                    req = req.query(&[("input_data", &input_json)]);
                }
                _ => {
                    // For POST/PUT/DELETE, send as form body
                    req = req
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .body(format!("input_data={}", urlencoding::encode(&input_json)));
                }
            }
        }

        let response = req.send().await.map_err(|e| {
            // Check for timeout specifically
            if e.is_timeout() {
                return GlassError::Timeout {
                    duration: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                    operation: format!("{} {}", method, path),
                };
            }
            GlassError::Http(e)
        })?;
        let status = response.status();

        // Handle HTTP-level errors
        if !status.is_success() {
            return Err(self.handle_http_error(status, response).await);
        }

        // Parse response body
        let body = response.text().await.map_err(GlassError::Http)?;

        tracing::trace!(body = %body, "SDP API response");

        // Parse as SdpResponse to check response_status
        let sdp_response: SdpResponse<T> =
            serde_json::from_str(&body).map_err(GlassError::Serialization)?;

        // Check SDP-level success and extract data
        sdp_response.into_result()
    }

    /// Makes a request to the SDP API with automatic retry for transient failures.
    ///
    /// This wraps `request_inner` with retry logic.
    async fn request<T>(
        &self,
        method: Method,
        path: &str,
        input_data: Option<serde_json::Value>,
    ) -> Result<T, GlassError>
    where
        T: serde::de::DeserializeOwned,
    {
        let operation = format!("{} {}", method, path);
        self.with_retry(&operation, || {
            self.request_inner(method.clone(), path, input_data.clone())
        })
        .await
    }

    /// Handles HTTP-level errors and converts to GlassError.
    ///
    /// Classifies errors into specific types for proper retry handling.
    async fn handle_http_error(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> GlassError {
        // Try to extract retry-after header for rate limiting
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        let body = response.text().await.unwrap_or_default();
        // Sanitize the body to ensure no API key leakage
        let body = GlassError::sanitize_message(&body, &self.api_key);
        // Truncate to avoid leaking verbose SDP internals
        let body = if body.len() > MAX_ERROR_BODY_LEN {
            format!("{}...[truncated]", &body[..MAX_ERROR_BODY_LEN])
        } else {
            body
        };

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => GlassError::Authentication,
            StatusCode::NOT_FOUND => GlassError::NotFound {
                id: "resource".to_string(),
            },
            StatusCode::TOO_MANY_REQUESTS => {
                tracing::warn!("Rate limited by SDP server");
                GlassError::RateLimited { retry_after }
            }
            StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT => {
                tracing::warn!(status = %status, "SDP server temporarily unavailable");
                GlassError::ServiceUnavailable { status }
            }
            _ => GlassError::HttpStatus { status, body },
        }
    }

    /// Lists requests with optional filtering and pagination.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for filtering and pagination
    ///
    /// # Returns
    ///
    /// A vector of request summaries matching the criteria.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Get first 10 open requests
    /// let params = ListParams::new()
    ///     .with_status("Open")
    ///     .with_limit(10);
    /// let requests = client.list_requests(params).await?;
    /// ```
    pub async fn list_requests(
        &self,
        params: ListParams,
    ) -> Result<Vec<RequestSummary>, GlassError> {
        let input_data = params.to_input_data();

        let response: ListRequestsResponse = self.get("/requests", Some(input_data)).await?;

        Ok(response.requests)
    }

    /// Gets full details of a single request.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique request ID
    ///
    /// # Returns
    ///
    /// The complete request details.
    ///
    /// # Errors
    ///
    /// Returns `GlassError::NotFound` if the request doesn't exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = client.get_request("12345").await?;
    /// println!("Subject: {}", request.display_subject());
    /// ```
    pub async fn get_request(&self, id: &str) -> Result<Request, GlassError> {
        Self::validate_id(id, "request_id")?;
        let path = format!("/requests/{}", id);

        let response: GetRequestResponse = self.get(&path, None).await.map_err(|e| {
            // Convert generic NotFound to one with the specific ID
            if matches!(e, GlassError::NotFound { .. }) {
                GlassError::NotFound { id: id.to_string() }
            } else {
                e
            }
        })?;

        Ok(response.request)
    }

    /// Gets notes for a request.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The unique request ID
    ///
    /// # Returns
    ///
    /// A vector of notes attached to the request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let notes = client.list_notes("12345").await?;
    /// for note in notes {
    ///     println!("{}: {}", note.display_created_by(), note.display_content());
    /// }
    /// ```
    pub async fn list_notes(&self, request_id: &str) -> Result<Vec<Note>, GlassError> {
        Self::validate_id(request_id, "request_id")?;
        let path = format!("/requests/{}/notes", request_id);

        let response: ListNotesResponse = self.get(&path, None).await.map_err(|e| {
            // Convert generic NotFound to one with the specific ID
            if matches!(e, GlassError::NotFound { .. }) {
                GlassError::NotFound {
                    id: request_id.to_string(),
                }
            } else {
                e
            }
        })?;

        Ok(response.notes)
    }

    /// Gets conversations (email replies) for a request.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The unique request ID
    ///
    /// # Returns
    ///
    /// A vector of conversations attached to the request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let conversations = client.list_conversations("12345").await?;
    /// for conv in conversations {
    ///     println!("{}: {}", conv.display_from(), conv.display_content());
    /// }
    /// ```
    pub async fn list_conversations(
        &self,
        request_id: &str,
    ) -> Result<Vec<Conversation>, GlassError> {
        Self::validate_id(request_id, "request_id")?;
        let path = format!("/requests/{}/conversations", request_id);

        let response: ListConversationsResponse = self.get(&path, None).await.map_err(|e| {
            // Convert generic NotFound to one with the specific ID
            if matches!(e, GlassError::NotFound { .. }) {
                GlassError::NotFound {
                    id: request_id.to_string(),
                }
            } else {
                e
            }
        })?;

        Ok(response.conversations)
    }

    /// Gets the content from a content_url.
    ///
    /// # Arguments
    ///
    /// * `content_url` - The relative URL path to fetch content from
    ///
    /// # Returns
    ///
    /// The content as HTML string wrapped in a JSON response.
    pub async fn get_content_from_url(&self, content_url: &str) -> Result<String, GlassError> {
        let content_url_owned = content_url.to_string();
        self.with_retry("get_content_from_url", || {
            self.get_content_from_url_inner(&content_url_owned)
        })
        .await
    }

    /// Inner implementation of content URL fetching (without retry wrapper).
    ///
    /// Validates that the constructed URL stays on the same host as the
    /// configured base URL to prevent SSRF attacks via crafted content_url values.
    async fn get_content_from_url_inner(&self, content_url: &str) -> Result<String, GlassError> {
        // The content_url is a relative path like /api/v3/requests/14992/notifications/88985
        // We need to construct the full URL properly
        let base = self.base_url.trim_end_matches("/api/v3");
        let url = format!("{}{}", base, content_url);

        // SSRF protection: validate the constructed URL's host matches the configured base URL
        let parsed_url = Url::parse(&url).map_err(|e| {
            GlassError::validation(format!("invalid content URL: {}", e))
        })?;
        let base_parsed = Url::parse(base).map_err(|e| {
            GlassError::validation(format!("invalid base URL: {}", e))
        })?;
        if parsed_url.host() != base_parsed.host() {
            return Err(GlassError::validation(format!(
                "content URL host mismatch: expected {:?}, got {:?}",
                base_parsed.host(),
                parsed_url.host()
            )));
        }

        let response = self
            .http
            .get(&url)
            .header("authtoken", &self.api_key)
            .header("Accept", SDP_ACCEPT_HEADER)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    return GlassError::Timeout {
                        duration: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                        operation: format!("GET {}", content_url),
                    };
                }
                GlassError::Http(e)
            })?;

        if !response.status().is_success() {
            return Err(self.handle_http_error(response.status(), response).await);
        }

        let body = response.text().await.map_err(GlassError::Http)?;

        // Try to parse as JSON and extract the content.
        // The response structure varies by content type:
        // - Notifications: { "notification": { "description": "..." } }
        // - Conversations: { "conversation": { "description": "..." } }
        // - Notes: { "note": { "description": "..." } }
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            // Try each known wrapper and field combination
            let content_paths: &[(&str, &str)] = &[
                ("notification", "description"),
                ("notification", "content"),
                ("conversation", "description"),
                ("note", "description"),
                ("note", "content"),
            ];
            for &(wrapper, field) in content_paths {
                if let Some(content) = json
                    .get(wrapper)
                    .and_then(|n| n.get(field))
                    .and_then(|c| c.as_str())
                {
                    return Ok(content.to_string());
                }
            }
        }

        // If not JSON or unexpected format, return the raw body
        Ok(body)
    }

    /// Gets conversations with their content populated.
    ///
    /// This is a convenience method that fetches conversations and then
    /// fetches the content for each one.
    pub async fn list_conversations_with_content(
        &self,
        request_id: &str,
    ) -> Result<Vec<Conversation>, GlassError> {
        let mut conversations = self.list_conversations(request_id).await?;

        // Fetch content for each conversation that has a content_url but no description
        for conv in &mut conversations {
            if conv.description.is_none() {
                if let Some(content_url) = &conv.content_url {
                    match self.get_content_from_url(content_url).await {
                        Ok(content) => {
                            conv.description = Some(content);
                        }
                        Err(e) => {
                            tracing::warn!(
                                conversation_id = %conv.id,
                                content_url = %content_url,
                                error = %e,
                                "Failed to fetch conversation content"
                            );
                        }
                    }
                }
            }
        }

        Ok(conversations)
    }

    /// Gets a single note by ID.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The unique request ID
    /// * `note_id` - The unique note ID
    ///
    /// # Returns
    ///
    /// The full note details including content.
    pub async fn get_note(&self, request_id: &str, note_id: &str) -> Result<Note, GlassError> {
        Self::validate_id(request_id, "request_id")?;
        Self::validate_id(note_id, "note_id")?;
        let path = format!("/requests/{}/notes/{}", request_id, note_id);

        // Make the request and parse the response
        // The single note endpoint returns { "note": { ... } }
        #[derive(Debug, serde::Deserialize)]
        struct GetNoteResponse {
            note: Note,
        }

        let response: GetNoteResponse = self.get(&path, None).await.map_err(|e| {
            if matches!(e, GlassError::NotFound { .. }) {
                GlassError::NotFound {
                    id: format!("note {} on request {}", note_id, request_id),
                }
            } else {
                e
            }
        })?;

        Ok(response.note)
    }

    /// Gets notes with their content populated.
    ///
    /// This method fetches the note list, then fetches each individual note
    /// to get the full content (SDP list endpoint doesn't include content).
    pub async fn list_notes_with_content(&self, request_id: &str) -> Result<Vec<Note>, GlassError> {
        let notes = self.list_notes(request_id).await?;

        // Fetch full details for each note (SDP list endpoint doesn't include content)
        let mut full_notes = Vec::with_capacity(notes.len());
        for note in notes {
            // If the note already has content, keep it as-is
            if note.description.is_some() {
                full_notes.push(note);
                continue;
            }

            // Fetch the individual note to get content
            match self.get_note(request_id, &note.id).await {
                Ok(full_note) => {
                    full_notes.push(full_note);
                }
                Err(e) => {
                    tracing::warn!(
                        note_id = %note.id,
                        request_id = %request_id,
                        error = %e,
                        "Failed to fetch note content, using partial note"
                    );
                    // Fall back to the partial note from the list
                    full_notes.push(note);
                }
            }
        }

        Ok(full_notes)
    }

    /// Lists technicians with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `group` - Optional group name to filter by
    /// * `limit` - Maximum number of technicians to return
    ///
    /// # Returns
    ///
    /// A vector of technicians matching the criteria.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Get first 50 technicians
    /// let technicians = client.list_technicians(None, Some(50)).await?;
    /// for tech in technicians {
    ///     println!("{}: {}", tech.id, tech.display_name());
    /// }
    /// ```
    pub async fn list_technicians(
        &self,
        group: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Technician>, GlassError> {
        let mut input_data = serde_json::Map::new();

        // Build list_info
        let mut list_info = serde_json::Map::new();
        if let Some(row_count) = limit {
            list_info.insert("row_count".to_string(), serde_json::json!(row_count));
        }
        input_data.insert(
            "list_info".to_string(),
            serde_json::Value::Object(list_info),
        );

        // Build search_criteria if group filter is provided
        if let Some(group_name) = group {
            let criteria = serde_json::json!([
                {
                    "field": "group.name",
                    "condition": "is",
                    "value": group_name
                }
            ]);
            input_data.insert("search_criteria".to_string(), criteria);
        }

        let response: ListTechniciansResponse = self
            .get("/technicians", Some(serde_json::Value::Object(input_data)))
            .await?;

        Ok(response.technicians)
    }

    // ========================================================================
    // Write operations (M4)
    // ========================================================================

    /// Creates a new request/ticket.
    ///
    /// # Arguments
    ///
    /// * `input` - The create request input containing subject and optional fields
    ///
    /// # Returns
    ///
    /// The created request with its assigned ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let input = CreateRequestInput {
    ///     subject: "Printer not working".to_string(),
    ///     priority: Some("High".to_string()),
    ///     ..Default::default()
    /// };
    /// let request = client.create_request(&input).await?;
    /// println!("Created ticket #{}", request.id);
    /// ```
    pub async fn create_request(&self, input: &CreateRequestInput) -> Result<Request, GlassError> {
        let mut request_data = serde_json::Map::new();

        // Required field
        request_data.insert("subject".to_string(), serde_json::json!(input.subject));

        // Optional fields
        if let Some(ref desc) = input.description {
            request_data.insert("description".to_string(), serde_json::json!(desc));
        }

        if let Some(ref email) = input.requester_email {
            request_data.insert(
                "requester".to_string(),
                serde_json::json!({"email_id": email}),
            );
        }

        if let Some(ref priority) = input.priority {
            request_data.insert(
                "priority".to_string(),
                serde_json::json!({"name": priority}),
            );
        }

        if let Some(ref category) = input.category {
            request_data.insert(
                "category".to_string(),
                serde_json::json!({"name": category}),
            );
        }

        if let Some(ref subcategory) = input.subcategory {
            request_data.insert(
                "subcategory".to_string(),
                serde_json::json!({"name": subcategory}),
            );
        }

        if let Some(ref item) = input.item {
            request_data.insert("item".to_string(), serde_json::json!({"name": item}));
        }

        if let Some(ref group) = input.group {
            request_data.insert("group".to_string(), serde_json::json!({"name": group}));
        }

        if let Some(ref tech_id) = input.technician_id {
            request_data.insert("technician".to_string(), serde_json::json!({"id": tech_id}));
        }

        let input_data = serde_json::json!({
            "request": request_data
        });

        let response: GetRequestResponse = self.post("/requests", input_data).await?;

        Ok(response.request)
    }

    /// Updates an existing request/ticket.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique request ID
    /// * `input` - The update input containing fields to modify
    ///
    /// # Returns
    ///
    /// The updated request.
    pub async fn update_request(
        &self,
        id: &str,
        input: &UpdateRequestInput,
    ) -> Result<Request, GlassError> {
        Self::validate_id(id, "request_id")?;
        let mut request_data = serde_json::Map::new();

        if let Some(ref subject) = input.subject {
            request_data.insert("subject".to_string(), serde_json::json!(subject));
        }

        if let Some(ref desc) = input.description {
            request_data.insert("description".to_string(), serde_json::json!(desc));
        }

        if let Some(ref priority) = input.priority {
            request_data.insert(
                "priority".to_string(),
                serde_json::json!({"name": priority}),
            );
        }

        if let Some(ref status) = input.status {
            request_data.insert("status".to_string(), serde_json::json!({"name": status}));
        }

        if let Some(ref category) = input.category {
            request_data.insert(
                "category".to_string(),
                serde_json::json!({"name": category}),
            );
        }

        if let Some(ref subcategory) = input.subcategory {
            request_data.insert(
                "subcategory".to_string(),
                serde_json::json!({"name": subcategory}),
            );
        }

        if let Some(ref group) = input.group {
            request_data.insert("group".to_string(), serde_json::json!({"name": group}));
        }

        if let Some(ref tech_id) = input.technician_id {
            request_data.insert("technician".to_string(), serde_json::json!({"id": tech_id}));
        }

        let input_data = serde_json::json!({
            "request": request_data
        });

        let path = format!("/requests/{}", id);
        let response: GetRequestResponse = self.put(&path, input_data).await?;

        Ok(response.request)
    }

    /// Closes a request/ticket.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique request ID
    /// * `closure_code` - Optional closure reason code
    /// * `comments` - Optional closure comments
    ///
    /// # Returns
    ///
    /// The closed request.
    pub async fn close_request(
        &self,
        id: &str,
        closure_code: Option<&str>,
        comments: Option<&str>,
    ) -> Result<Request, GlassError> {
        Self::validate_id(id, "request_id")?;
        let mut request_data = serde_json::Map::new();

        // Build closure_info
        let mut closure_info = serde_json::Map::new();

        if let Some(code) = closure_code {
            closure_info.insert(
                "closure_code".to_string(),
                serde_json::json!({"name": code}),
            );
        }

        if let Some(comment) = comments {
            closure_info.insert("closure_comments".to_string(), serde_json::json!(comment));
        }

        if !closure_info.is_empty() {
            request_data.insert(
                "closure_info".to_string(),
                serde_json::Value::Object(closure_info),
            );
        }

        let input_data = serde_json::json!({
            "request": request_data
        });

        let path = format!("/requests/{}/close", id);
        let response: GetRequestResponse = self.put(&path, input_data).await?;

        Ok(response.request)
    }

    /// Adds a note to a request/ticket.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The unique request ID
    /// * `content` - The note content
    /// * `show_to_requester` - Whether to show the note to the requester
    /// * `notify_technician` - Whether to notify the assigned technician
    ///
    /// # Returns
    ///
    /// The created note.
    pub async fn add_note(
        &self,
        request_id: &str,
        content: &str,
        show_to_requester: Option<bool>,
        notify_technician: Option<bool>,
    ) -> Result<Note, GlassError> {
        Self::validate_id(request_id, "request_id")?;
        let note_request = CreateNoteRequest::new(content);

        let note_request = if let Some(show) = show_to_requester {
            note_request.with_show_to_requester(show)
        } else {
            note_request
        };

        let note_request = if let Some(notify) = notify_technician {
            note_request.with_notify_technician(notify)
        } else {
            note_request
        };

        let input_data = serde_json::json!({
            "note": note_request
        });

        let path = format!("/requests/{}/notes", request_id);
        let response: AddNoteResponse = self.post(&path, input_data).await?;

        Ok(response.note)
    }

    /// Assigns a request/ticket to a technician and/or group.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique request ID
    /// * `technician_id` - Optional technician ID to assign
    /// * `group` - Optional group name to assign
    ///
    /// # Returns
    ///
    /// The updated request.
    pub async fn assign_request(
        &self,
        id: &str,
        technician_id: Option<&str>,
        group: Option<&str>,
    ) -> Result<Request, GlassError> {
        Self::validate_id(id, "request_id")?;
        if let Some(tech_id) = technician_id {
            Self::validate_id(tech_id, "technician_id")?;
        }
        let mut request_data = serde_json::Map::new();

        if let Some(tech_id) = technician_id {
            request_data.insert("technician".to_string(), serde_json::json!({"id": tech_id}));
        }

        if let Some(group_name) = group {
            request_data.insert("group".to_string(), serde_json::json!({"name": group_name}));
        }

        let input_data = serde_json::json!({
            "request": request_data
        });

        let path = format!("/requests/{}", id);
        let response: GetRequestResponse = self.put(&path, input_data).await?;

        Ok(response.request)
    }

    // ========================================================================
    // Private helper methods for HTTP verbs
    // ========================================================================

    /// Makes a POST request to the SDP API.
    async fn post<T>(&self, path: &str, input_data: serde_json::Value) -> Result<T, GlassError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.request::<T>(Method::POST, path, Some(input_data))
            .await
    }

    /// Makes a PUT request to the SDP API.
    async fn put<T>(&self, path: &str, input_data: serde_json::Value) -> Result<T, GlassError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.request::<T>(Method::PUT, path, Some(input_data)).await
    }
}

/// Parameters for listing requests.
///
/// Use the builder methods to construct filter criteria.
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    /// Pagination settings.
    list_info: ListInfo,

    /// Search criteria for filtering.
    search_criteria: SearchCriteria,
}

impl ListParams {
    /// Creates empty list parameters (returns all requests with default pagination).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of results to return.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.list_info.row_count = Some(limit);
        self
    }

    /// Sets the starting offset for pagination.
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.list_info.start_index = Some(offset);
        self
    }

    /// Filters by status name (e.g., "Åben", "Lukket").
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria
            .criteria
            .push(SearchCriterion::is("status.name", status));
        self
    }

    /// Filters to exclude closed/completed statuses.
    /// Excludes: Lukket, Annulleret, Udført (afventer godkendelse)
    pub fn with_open_only(mut self) -> Self {
        use crate::models::SearchCriterion;

        // Use "is not" condition to exclude closed statuses
        self.search_criteria.criteria.push(SearchCriterion {
            field: "status.name".to_string(),
            condition: "is not".to_string(),
            value: serde_json::Value::String("Lukket".to_string()),
            logical_operator: None,
        });

        self.search_criteria.criteria.push(SearchCriterion {
            field: "status.name".to_string(),
            condition: "is not".to_string(),
            value: serde_json::Value::String("Annulleret".to_string()),
            logical_operator: None,
        });

        self.search_criteria.criteria.push(SearchCriterion {
            field: "status.name".to_string(),
            condition: "is not".to_string(),
            value: serde_json::Value::String("Udført, afventer godkendelse".to_string()),
            logical_operator: None,
        });

        self
    }

    /// Filters by priority name (e.g., "High", "Low").
    pub fn with_priority(mut self, priority: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria
            .criteria
            .push(SearchCriterion::is("priority.name", priority));
        self
    }

    /// Filters by technician name.
    pub fn with_technician(mut self, technician: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria
            .criteria
            .push(SearchCriterion::is("technician.name", technician));
        self
    }

    /// Filters by requester name.
    pub fn with_requester(mut self, requester: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria
            .criteria
            .push(SearchCriterion::is("requester.name", requester));
        self
    }

    /// Filters by created time after a date (ISO 8601: YYYY-MM-DD).
    pub fn with_created_after(mut self, date: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria.criteria.push(SearchCriterion {
            field: "created_time".to_string(),
            condition: "greater than".to_string(),
            value: serde_json::Value::String(date.into()),
            logical_operator: None,
        });
        self
    }

    /// Filters by created time before a date (ISO 8601: YYYY-MM-DD).
    pub fn with_created_before(mut self, date: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria.criteria.push(SearchCriterion {
            field: "created_time".to_string(),
            condition: "less than".to_string(),
            value: serde_json::Value::String(date.into()),
            logical_operator: None,
        });
        self
    }

    /// Searches by subject (partial match).
    pub fn with_subject_contains(mut self, subject: impl Into<String>) -> Self {
        use crate::models::SearchCriterion;

        self.search_criteria
            .criteria
            .push(SearchCriterion::contains("subject", subject));
        self
    }

    /// Requests the total count along with results.
    pub fn with_total_count(mut self) -> Self {
        self.list_info.get_total_count = Some(true);
        self
    }

    /// Converts parameters to the input_data JSON structure.
    fn to_input_data(&self) -> serde_json::Value {
        let mut data = serde_json::Map::new();

        // Build list_info object
        let mut list_info =
            serde_json::to_value(&self.list_info).unwrap_or_else(|_| serde_json::json!({}));

        // SDP expects search_criteria INSIDE list_info.
        // All criteria except the last need a logical_operator ("AND").
        if !self.search_criteria.is_empty() {
            let mut criteria = self.search_criteria.criteria.clone();
            for i in 0..criteria.len().saturating_sub(1) {
                if criteria[i].logical_operator.is_none() {
                    criteria[i].logical_operator = Some("AND".to_string());
                }
            }
            // Last criterion should not have a logical_operator
            if let Some(last) = criteria.last_mut() {
                last.logical_operator = None;
            }
            if let serde_json::Value::Object(ref mut map) = list_info {
                map.insert(
                    "search_criteria".to_string(),
                    serde_json::to_value(&criteria).unwrap_or_else(|_| serde_json::json!([])),
                );
            }
        }

        data.insert("list_info".to_string(), list_info);
        serde_json::Value::Object(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_base_url() {
        assert_eq!(
            SdpClient::normalize_base_url("https://example.com"),
            "https://example.com/api/v3"
        );
        assert_eq!(
            SdpClient::normalize_base_url("https://example.com/"),
            "https://example.com/api/v3"
        );
        assert_eq!(
            SdpClient::normalize_base_url("https://example.com/api/v3"),
            "https://example.com/api/v3"
        );
        assert_eq!(
            SdpClient::normalize_base_url("https://example.com/api/v3/"),
            "https://example.com/api/v3"
        );
        assert_eq!(
            SdpClient::normalize_base_url("https://example.com/api"),
            "https://example.com/api/v3"
        );
    }

    #[test]
    fn test_list_params_default() {
        let params = ListParams::new();
        let input_data = params.to_input_data();

        assert!(input_data.is_object());
        assert!(input_data.get("list_info").is_some());
    }

    #[test]
    fn test_list_params_with_limit() {
        let params = ListParams::new().with_limit(10);
        let input_data = params.to_input_data();

        let list_info = input_data.get("list_info").unwrap();
        assert_eq!(list_info.get("row_count").unwrap(), 10);
    }

    #[test]
    fn test_list_params_with_status() {
        let params = ListParams::new().with_status("Open");
        let input_data = params.to_input_data();

        // search_criteria should be inside list_info
        let list_info = input_data.get("list_info").unwrap();
        let criteria = list_info.get("search_criteria").unwrap();
        assert!(criteria.is_array());
        let first = criteria.as_array().unwrap().first().unwrap();
        assert_eq!(first.get("field").unwrap(), "status.name");
        assert_eq!(first.get("value").unwrap(), "Open");
    }

    #[test]
    fn test_list_params_multiple_criteria() {
        let params = ListParams::new().with_status("Open").with_priority("High");
        let input_data = params.to_input_data();

        // search_criteria should be inside list_info
        let list_info = input_data.get("list_info").unwrap();
        let criteria = list_info.get("search_criteria").unwrap();
        let arr = criteria.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        // Both criteria should be present
        assert_eq!(arr[0].get("field").unwrap(), "status.name");
        assert_eq!(arr[1].get("field").unwrap(), "priority.name");
    }

    #[test]
    fn test_validate_id_valid() {
        assert!(SdpClient::validate_id("12345", "test").is_ok());
        assert!(SdpClient::validate_id("0", "test").is_ok());
        assert!(SdpClient::validate_id("999999999", "test").is_ok());
    }

    #[test]
    fn test_validate_id_rejects_empty() {
        let err = SdpClient::validate_id("", "request_id").unwrap_err();
        assert!(err.to_string().contains("request_id"));
        assert!(err.to_string().contains("numeric"));
    }

    #[test]
    fn test_validate_id_rejects_non_numeric() {
        assert!(SdpClient::validate_id("abc", "id").is_err());
        assert!(SdpClient::validate_id("123abc", "id").is_err());
        assert!(SdpClient::validate_id("12/34", "id").is_err());
        assert!(SdpClient::validate_id("../etc/passwd", "id").is_err());
        assert!(SdpClient::validate_id("12 34", "id").is_err());
        assert!(SdpClient::validate_id("-1", "id").is_err());
    }

    /// Creates an SdpClient for unit tests without requiring Config/env vars.
    fn test_client() -> SdpClient {
        SdpClient {
            http: Client::new(),
            base_url: "https://example.com/api/v3".to_string(),
            api_key: "test_key".to_string(),
        }
    }

    #[test]
    fn test_request_web_url_encodes_id() {
        let client = test_client();
        let url = client.request_web_url("12345");
        assert!(url.contains("woID=12345"));

        // Verify special characters are encoded
        let url = client.request_web_url("123&evil=true");
        assert!(!url.contains("&evil=true"));
        assert!(url.contains("woID=123%26evil%3Dtrue"));
    }
}
