use std::fmt;

/// VKT Error Types
#[derive(Debug)]
pub enum VktError {
    /// Configuration error
    Config(String),
    /// API call error
    Api(String),
    /// Network error
    Network(String),
    /// IO error
    Io(std::io::Error),
    /// Parameter validation error
    Validation(String),
    /// Authentication failed (401)
    AuthInvalid(String),
    /// Permission denied (403)
    PermissionDenied(String),
    /// Rate limited (429)
    RateLimited(String),
    /// Resource not found (404)
    ApiNotFound(String),
    /// Resource conflict (409)
    Conflict(String),
}

impl fmt::Display for VktError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VktError::Config(msg) => write!(f, "Config error: {}", msg),
            VktError::Api(msg) => write!(f, "API error: {}", msg),
            VktError::Network(msg) => write!(f, "Network error: {}", msg),
            VktError::Io(err) => write!(f, "IO error: {}", err),
            VktError::Validation(msg) => write!(f, "Validation error: {}", msg),
            VktError::AuthInvalid(msg) => write!(f, "Authentication failed: {}", msg),
            VktError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            VktError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            VktError::ApiNotFound(msg) => write!(f, "Resource not found: {}", msg),
            VktError::Conflict(msg) => write!(f, "Resource conflict: {}", msg),
        }
    }
}

impl std::error::Error for VktError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VktError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for VktError {
    fn from(err: std::io::Error) -> Self {
        VktError::Io(err)
    }
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
