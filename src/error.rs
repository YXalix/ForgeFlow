use thiserror::Error;

/// VKT Error Types
#[derive(Error, Debug)]
pub enum VktError {
    /// Configuration error
    #[error("Config error: {0}")]
    Config(String),

    /// API call error
    #[error("API error: {0}")]
    Api(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parameter validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication failed (401)
    #[error("Authentication failed: {0}")]
    AuthInvalid(String),

    /// Permission denied (403)
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Rate limited (429)
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// Resource not found (404)
    #[error("Resource not found: {0}")]
    ApiNotFound(String),

    /// Resource conflict (409)
    #[error("Resource conflict: {0}")]
    Conflict(String),
}

impl From<reqwest::Error> for VktError {
    fn from(err: reqwest::Error) -> Self {
        VktError::Network(err.to_string())
    }
}

impl VktError {
    /// Check if error is retryable (temporary)
    pub fn is_retryable(&self) -> bool {
        matches!(self, VktError::Network(_) | VktError::RateLimited(_))
    }

    /// Check if error is authentication-related
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            VktError::AuthInvalid(_) | VktError::PermissionDenied(_)
        )
    }

    /// Check if error is resource not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, VktError::ApiNotFound(_))
    }
}

/// VKT Result type
pub type Result<T> = std::result::Result<T, VktError>;
