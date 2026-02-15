//! Provider Factory
//!
//! Creates the appropriate ForgeProvider implementation based on configuration.

use crate::config::{Config, ProviderType};
use crate::error::{Result, VktError};

use super::gitcode::GitCodeProvider;
use super::traits::ForgeProvider;

/// Create a provider based on configuration
///
/// # Arguments
/// * `config` - The VKT configuration
///
/// # Returns
/// A boxed ForgeProvider implementation
///
/// # Errors
/// Returns an error if the provider type is not supported
pub fn create_provider(config: &Config) -> Result<Box<dyn ForgeProvider>> {
    match config.remote.provider_type() {
        ProviderType::GitCode => {
            let provider = GitCodeProvider::new(config)?;
            Ok(Box::new(provider))
        }
        ProviderType::GitLab => Err(VktError::Config(
            "GitLab provider not yet implemented".to_string(),
        )),
        ProviderType::GitHub => Err(VktError::Config(
            "GitHub provider not yet implemented".to_string(),
        )),
        ProviderType::Unknown(provider) => Err(VktError::Config(format!(
            "Unknown provider: {}. Supported providers: GitCode, GitLab, GitHub",
            provider
        ))),
    }
}

/// Auto-detect provider type from API URL
///
/// # Arguments
/// * `api_url` - The API URL to analyze
///
/// # Returns
/// The detected ProviderType
pub fn detect_provider(api_url: &str) -> ProviderType {
    let url_lower = api_url.to_lowercase();

    if url_lower.contains("gitcode.com") {
        ProviderType::GitCode
    } else if url_lower.contains("gitlab") || url_lower.contains("git-lab") {
        ProviderType::GitLab
    } else if url_lower.contains("github.com") {
        ProviderType::GitHub
    } else {
        ProviderType::Unknown("unknown".to_string())
    }
}

#[cfg(test)]
mod tests {}
