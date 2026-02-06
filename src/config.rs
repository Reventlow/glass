//! Configuration management for the Glass MCP server.
//!
//! This module handles loading configuration from environment variables,
//! with validation to ensure all required values are present.

use crate::error::GlassError;
use std::env;

/// Configuration for connecting to ServiceDesk Plus.
///
/// All fields are required and loaded from environment variables.
/// The API key is stored but never logged or exposed in error messages.
#[derive(Clone)]
pub struct Config {
    /// Base URL for the SDP instance (e.g., `https://servicedesk.example.com`).
    pub base_url: String,

    /// Technician API key for authentication.
    /// This value must never be logged or included in error messages.
    pub api_key: String,
}

impl Config {
    /// Loads configuration from environment variables.
    ///
    /// # Required Environment Variables
    ///
    /// - `SDP_BASE_URL`: The base URL of the ServiceDesk Plus instance
    /// - `SDP_API_KEY`: The technician API key for authentication
    ///
    /// # Errors
    ///
    /// Returns `GlassError::Config` if any required variable is missing
    /// or if values fail validation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// dotenvy::dotenv().ok();
    /// let config = Config::from_env()?;
    /// ```
    pub fn from_env() -> Result<Self, GlassError> {
        let base_url = Self::get_required_env("SDP_BASE_URL")?;
        let api_key = Self::get_required_env("SDP_API_KEY")?;

        // Validate base URL format
        let base_url = Self::validate_base_url(base_url)?;

        // Validate API key is not empty or placeholder
        Self::validate_api_key(&api_key)?;

        Ok(Config { base_url, api_key })
    }

    /// Gets a required environment variable, returning an error if missing or empty.
    fn get_required_env(name: &str) -> Result<String, GlassError> {
        env::var(name)
            .map_err(|_| GlassError::missing_env(name))
            .and_then(|value| {
                if value.trim().is_empty() {
                    Err(GlassError::missing_env(name))
                } else {
                    Ok(value)
                }
            })
    }

    /// Validates and normalizes the base URL.
    fn validate_base_url(url: String) -> Result<String, GlassError> {
        let url = url.trim().to_string();

        // Remove trailing slash for consistency
        let url = url.trim_end_matches('/').to_string();

        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(GlassError::invalid_config(
                "SDP_BASE_URL must start with http:// or https://",
            ));
        }

        Ok(url)
    }

    /// Validates the API key is not a placeholder value.
    fn validate_api_key(key: &str) -> Result<(), GlassError> {
        let key_lower = key.to_lowercase();
        let placeholder_patterns = [
            "your_api_key",
            "your_key",
            "placeholder",
            "xxx",
            "changeme",
        ];

        for pattern in placeholder_patterns {
            if key_lower.contains(pattern) {
                return Err(GlassError::invalid_config(
                    "SDP_API_KEY appears to be a placeholder value",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests that modify environment variables should not run in parallel.
    // Use `cargo test -- --test-threads=1` for full integration tests.

    #[test]
    fn test_validate_base_url_removes_trailing_slash() {
        let result = Config::validate_base_url("https://example.com/".to_string()).unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn test_validate_base_url_requires_scheme() {
        let result = Config::validate_base_url("example.com".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_api_key_rejects_placeholder() {
        let result = Config::validate_api_key("your_api_key_here");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_api_key_accepts_real_key() {
        let result = Config::validate_api_key("abc123def456");
        assert!(result.is_ok());
    }
}
