//! Error types for the Glass MCP server.
//!
//! This module defines `GlassError`, the unified error type used throughout
//! the application for consistent error handling and propagation.
//!
//! # Security
//!
//! All error messages are sanitized to ensure API keys are never leaked
//! in logs or error responses. Use `sanitize_message()` when constructing
//! error messages from external sources.

use std::time::Duration;
use thiserror::Error;

/// Common SDP API error codes.
pub mod codes {
    /// Success response.
    pub const SUCCESS: u32 = 2000;
    /// Authentication failure.
    pub const AUTH_FAILED: u32 = 4001;
    /// Resource not found.
    pub const NOT_FOUND: u32 = 4005;
    /// Rate limit exceeded.
    pub const RATE_LIMITED: u32 = 4029;
    /// Internal server error.
    pub const SERVER_ERROR: u32 = 5000;
}

/// Unified error type for all Glass operations.
///
/// Each variant provides specific context about the failure, enabling
/// meaningful error messages without leaking sensitive information
/// like API keys.
#[derive(Error, Debug)]
pub enum GlassError {
    /// Configuration error - missing or invalid environment variables.
    #[error("configuration error: {0}")]
    Config(String),

    /// HTTP request failed during transmission.
    #[error("HTTP request failed: {0}")]
    Http(#[source] reqwest::Error),

    /// HTTP client initialization failed.
    #[error("HTTP client error: {0}")]
    HttpClient(#[source] reqwest::Error),

    /// HTTP response returned a non-success status code.
    #[error("HTTP {status}: {body}")]
    HttpStatus {
        /// The HTTP status code returned.
        status: reqwest::StatusCode,
        /// The response body, potentially containing error details.
        body: String,
    },

    /// Request timed out.
    #[error("request timed out after {duration:?} - the server may be slow or unreachable")]
    Timeout {
        /// How long we waited before timing out.
        duration: Duration,
        /// The operation that timed out.
        operation: String,
    },

    /// Rate limited by the server (HTTP 429).
    #[error("rate limited by server - please wait before retrying")]
    RateLimited {
        /// Suggested retry delay, if provided by server.
        retry_after: Option<Duration>,
    },

    /// Server temporarily unavailable (HTTP 502/503/504).
    #[error("service temporarily unavailable ({status}) - will retry automatically")]
    ServiceUnavailable {
        /// The specific status code.
        status: reqwest::StatusCode,
    },

    /// ServiceDesk Plus API returned an error response.
    #[error("SDP API error {code}: {message}")]
    SdpApi {
        /// SDP-specific error code.
        code: u32,
        /// Human-readable error message from SDP.
        message: String,
        /// The request ID this error relates to, if applicable.
        request_id: Option<String>,
    },

    /// JSON serialization or deserialization failed.
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Requested resource was not found.
    #[error("request not found: {id}")]
    NotFound {
        /// The ID of the resource that was not found.
        id: String,
    },

    /// Authentication failed - likely an invalid API key.
    #[error("authentication failed - check SDP_API_KEY")]
    Authentication,

    /// Input validation failed.
    #[error("validation error: {0}")]
    Validation(String),

    /// Connection test failed.
    #[error("connection test failed: {message}")]
    ConnectionTest {
        /// Details about why the connection test failed.
        message: String,
    },
}

impl GlassError {
    /// Creates a configuration error for a missing environment variable.
    pub fn missing_env(var_name: &str) -> Self {
        GlassError::Config(format!(
            "missing required environment variable: {}",
            var_name
        ))
    }

    /// Creates a configuration error for an invalid value.
    pub fn invalid_config(message: impl Into<String>) -> Self {
        GlassError::Config(message.into())
    }

    /// Creates a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        GlassError::Validation(message.into())
    }

    /// Creates a not found error for a request ID.
    pub fn not_found(id: impl Into<String>) -> Self {
        GlassError::NotFound { id: id.into() }
    }

    /// Creates a timeout error.
    pub fn timeout(duration: Duration, operation: impl Into<String>) -> Self {
        GlassError::Timeout {
            duration,
            operation: operation.into(),
        }
    }

    /// Creates an SDP API error with optional request context.
    pub fn sdp_api(code: u32, message: impl Into<String>, request_id: Option<String>) -> Self {
        GlassError::SdpApi {
            code,
            message: message.into(),
            request_id,
        }
    }

    /// Creates a connection test error.
    pub fn connection_test(message: impl Into<String>) -> Self {
        GlassError::ConnectionTest {
            message: message.into(),
        }
    }

    /// Returns true if this error is transient and the operation should be retried.
    ///
    /// Retryable errors include:
    /// - Rate limiting (HTTP 429)
    /// - Service unavailable (HTTP 502, 503, 504)
    /// - Timeouts (may succeed on retry)
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            GlassError::RateLimited { .. } => true,
            GlassError::ServiceUnavailable { .. } => true,
            GlassError::Timeout { .. } => true,
            GlassError::Http(e) => {
                // Check if it's a timeout or connection error
                e.is_timeout() || e.is_connect()
            }
            GlassError::HttpStatus { status, .. } => {
                // 429 (rate limit) and 5xx server errors are retryable
                status.as_u16() == 429 || status.is_server_error()
            }
            _ => false,
        }
    }

    /// Returns true if this is a rate limit error, indicating we should back off.
    #[must_use]
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, GlassError::RateLimited { .. })
            || matches!(self, GlassError::HttpStatus { status, .. } if status.as_u16() == 429)
    }

    /// Returns the suggested delay before retry, if any.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            GlassError::RateLimited { retry_after } => *retry_after,
            GlassError::ServiceUnavailable { .. } => Some(Duration::from_millis(500)),
            GlassError::Timeout { .. } => Some(Duration::from_millis(100)),
            _ => None,
        }
    }

    /// Sanitizes an error message to remove any occurrence of the API key.
    ///
    /// This is critical for security - API keys must never appear in logs,
    /// error messages, or responses to users.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to sanitize
    /// * `api_key` - The API key to strip from the message
    ///
    /// # Returns
    ///
    /// The message with any occurrence of the API key replaced with `[REDACTED]`
    #[must_use]
    pub fn sanitize_message(message: &str, api_key: &str) -> String {
        if api_key.is_empty() {
            return message.to_string();
        }
        message.replace(api_key, "[REDACTED]")
    }

    /// Creates a sanitized version of this error's display message.
    ///
    /// Use this when you need to include error details in logs or responses
    /// and want to ensure no sensitive data is leaked.
    #[must_use]
    pub fn sanitized_display(&self, api_key: &str) -> String {
        Self::sanitize_message(&self.to_string(), api_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_env_error() {
        let err = GlassError::missing_env("SDP_API_KEY");
        assert!(err.to_string().contains("SDP_API_KEY"));
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn test_validation_error() {
        let err = GlassError::validation("subject is required");
        assert_eq!(err.to_string(), "validation error: subject is required");
    }

    #[test]
    fn test_not_found_error() {
        let err = GlassError::not_found("12345");
        assert_eq!(err.to_string(), "request not found: 12345");
    }

    #[test]
    fn test_timeout_error() {
        let err = GlassError::timeout(Duration::from_secs(30), "list_requests");
        let msg = err.to_string();
        assert!(msg.contains("timed out"));
        assert!(msg.contains("30s"));
    }

    #[test]
    fn test_is_retryable_rate_limited() {
        let err = GlassError::RateLimited { retry_after: None };
        assert!(err.is_retryable());
        assert!(err.is_rate_limit());
    }

    #[test]
    fn test_is_retryable_service_unavailable() {
        let err = GlassError::ServiceUnavailable {
            status: reqwest::StatusCode::BAD_GATEWAY,
        };
        assert!(err.is_retryable());
        assert!(!err.is_rate_limit());
    }

    #[test]
    fn test_is_retryable_not_found() {
        let err = GlassError::not_found("123");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_validation() {
        let err = GlassError::validation("invalid input");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_sanitize_message_removes_api_key() {
        let api_key = "super_secret_key_12345";
        let message = format!("Error connecting with key {} to server", api_key);
        let sanitized = GlassError::sanitize_message(&message, api_key);
        assert!(!sanitized.contains(api_key));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_message_empty_key() {
        let message = "Some error message";
        let sanitized = GlassError::sanitize_message(message, "");
        assert_eq!(sanitized, message);
    }

    #[test]
    fn test_sanitize_message_no_match() {
        let message = "Some error message";
        let sanitized = GlassError::sanitize_message(message, "not_present");
        assert_eq!(sanitized, message);
    }

    #[test]
    fn test_sdp_api_error_with_request_id() {
        let err = GlassError::sdp_api(4005, "Resource not found", Some("12345".to_string()));
        let msg = err.to_string();
        assert!(msg.contains("4005"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_retry_after_rate_limited() {
        let err = GlassError::RateLimited {
            retry_after: Some(Duration::from_secs(5)),
        };
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_retry_after_service_unavailable() {
        let err = GlassError::ServiceUnavailable {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        };
        assert_eq!(err.retry_after(), Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_connection_test_error() {
        let err = GlassError::connection_test("Could not reach server");
        let msg = err.to_string();
        assert!(msg.contains("connection test failed"));
        assert!(msg.contains("Could not reach server"));
    }
}
