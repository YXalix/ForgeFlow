//! Configuration management module
//!
//! Handles loading and validation of TOML configuration files

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Result, VktError};

/// Environment variable prefix
const ENV_PREFIX: &str = "VKT";

/// Main configuration struct
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// User configuration
    pub user: UserConfig,
    /// Remote repository configuration
    pub remote: RemoteConfig,
    /// Repository configuration
    pub repo: RepoConfig,
    /// Template configuration
    #[serde(default)]
    pub template: TemplateConfig,
}

/// User configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserConfig {
    /// User name
    pub name: String,
    /// Email address
    pub email: String,
    /// Automatically add Signed-off-by
    #[serde(default)]
    pub auto_signoff: bool,
}

/// Provider type
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    GitCode,
    GitLab,
    GitHub,
    Unknown(String),
}

impl ProviderType {
    /// Parse provider type from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "gitcode" => ProviderType::GitCode,
            "gitlab" => ProviderType::GitLab,
            "github" => ProviderType::GitHub,
            other => ProviderType::Unknown(other.to_string()),
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &str {
        match self {
            ProviderType::GitCode => "gitcode",
            ProviderType::GitLab => "gitlab",
            ProviderType::GitHub => "github",
            ProviderType::Unknown(s) => s.as_str(),
        }
    }
}

/// Remote repository configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteConfig {
    /// Provider (Gitcode, GitLab, GitHub, etc.)
    pub provider: String,
    /// API URL
    pub api_url: String,
    /// Access token
    pub token: String,
}

impl RemoteConfig {
    /// Get provider type enum
    pub fn provider_type(&self) -> ProviderType {
        ProviderType::parse(&self.provider)
    }
}

/// Auto-detect provider type from API URL
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

/// Repository configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoConfig {
    /// Project ID (e.g., "owner/repo")
    pub project_id: String,
    /// Default branch
    #[serde(default = "default_branch")]
    pub default_branch: String,
}

/// Template configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TemplateConfig {
    /// PR title prefix
    #[serde(default)]
    pub pr_prefix: String,
}

fn default_branch() -> String {
    "main".to_string()
}

impl Config {
    /// Get default configuration file path (~/.config/vkt/config.toml)
    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| VktError::Config("Cannot get configuration directory".to_string()))?;
        Ok(config_dir.join("vkt").join("config.toml"))
    }

    /// Load configuration from file
    pub fn parse_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| VktError::Config(format!("Failed to read configuration file: {}", e)))?;
        Self::parse_from_str(&content)
    }

    /// Load configuration from string
    pub fn parse_from_str(content: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(content)
            .map_err(|e| VktError::Config(format!("Failed to parse configuration file: {}", e)))?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Load default configuration
    pub fn load() -> Result<Self> {
        let path = Self::default_path()?;
        Self::parse_from_file(&path)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // User configuration
        if let Ok(name) = std::env::var(format!("{}_USER_NAME", ENV_PREFIX)) {
            self.user.name = name;
        }
        if let Ok(email) = std::env::var(format!("{}_USER_EMAIL", ENV_PREFIX)) {
            self.user.email = email;
        }
        if let Ok(signoff) = std::env::var(format!("{}_USER_AUTO_SIGNOFF", ENV_PREFIX)) {
            self.user.auto_signoff = signoff.parse().unwrap_or(self.user.auto_signoff);
        }

        // Remote configuration
        if let Ok(provider) = std::env::var(format!("{}_REMOTE_PROVIDER", ENV_PREFIX)) {
            self.remote.provider = provider;
        }
        if let Ok(api_url) = std::env::var(format!("{}_REMOTE_API_URL", ENV_PREFIX)) {
            self.remote.api_url = api_url;
        }
        if let Ok(token) = std::env::var(format!("{}_REMOTE_TOKEN", ENV_PREFIX)) {
            self.remote.token = token;
        }

        // Repository configuration
        if let Ok(project_id) = std::env::var(format!("{}_REPO_PROJECT_ID", ENV_PREFIX)) {
            self.repo.project_id = project_id;
        }
        if let Ok(default_branch) = std::env::var(format!("{}_REPO_DEFAULT_BRANCH", ENV_PREFIX)) {
            self.repo.default_branch = default_branch;
        }

        // Template configuration
        if let Ok(pr_prefix) = std::env::var(format!("{}_TEMPLATE_PR_PREFIX", ENV_PREFIX)) {
            self.template.pr_prefix = pr_prefix;
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // User configuration validation
        if self.user.name.is_empty() {
            return Err(VktError::Config("User name cannot be empty".to_string()));
        }
        if self.user.email.is_empty() {
            return Err(VktError::Config("User email cannot be empty".to_string()));
        }
        if !Self::is_valid_email(&self.user.email) {
            return Err(VktError::Config(format!(
                "Invalid email format: {}",
                self.user.email
            )));
        }

        // Remote configuration validation
        if self.remote.provider.is_empty() {
            return Err(VktError::Config("Provider cannot be empty".to_string()));
        }
        if self.remote.api_url.is_empty() {
            return Err(VktError::Config("API URL cannot be empty".to_string()));
        }
        if !Self::is_valid_url(&self.remote.api_url) {
            return Err(VktError::Config(format!(
                "Invalid API URL format: {}",
                self.remote.api_url
            )));
        }
        if self.remote.token.is_empty() {
            return Err(VktError::Config("Access token cannot be empty".to_string()));
        }

        // Repository configuration validation
        if self.repo.project_id.is_empty() {
            return Err(VktError::Config("Project ID cannot be empty".to_string()));
        }
        if !self.repo.project_id.contains('/') {
            return Err(VktError::Config(
                "Project ID format should be 'owner/repo'".to_string(),
            ));
        }

        Ok(())
    }

    /// Simple email format validation
    fn is_valid_email(email: &str) -> bool {
        email.contains('@')
            && email.contains('.')
            && !email.starts_with('@')
            && !email.ends_with('.')
            && email.len() > 5
    }

    /// Simple URL format validation
    fn is_valid_url(url: &str) -> bool {
        (url.starts_with("http://") || url.starts_with("https://")) && url.len() > 10
    }

    /// Generate configuration example
    pub fn example() -> String {
        r#"# VKT Configuration File Example
# Location: ~/.config/vkt/config.toml

[user]
name = "John Doe"
email = "john.doe@example.com"
auto_signoff = true

[remote]
provider = "Gitcode"
api_url = "https://gitcode.com/api/v4"
token = "your-api-token-here"

[repo]
project_id = "owner/repo"
default_branch = "main"

[template]
pr_prefix = "[VIRT-TOOL]"
"#
        .to_string()
    }

    /// Create configuration directory
    pub fn ensure_config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| VktError::Config("Cannot get configuration directory".to_string()))?
            .join("vkt");

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| {
                VktError::Config(format!("Failed to create configuration directory: {}", e))
            })?;
        }

        Ok(config_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_config() -> Config {
        Config {
            user: UserConfig {
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
                auto_signoff: true,
            },
            remote: RemoteConfig {
                provider: "Gitcode".to_string(),
                api_url: "https://api.example.com".to_string(),
                token: "token123".to_string(),
            },
            repo: RepoConfig {
                project_id: "owner/repo".to_string(),
                default_branch: "main".to_string(),
            },
            template: TemplateConfig::default(),
        }
    }

    #[test]
    fn test_config_validate_success() {
        let config = create_valid_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_name() {
        let mut config = create_valid_config();
        config.user.name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_email() {
        let mut config = create_valid_config();
        config.user.email = "invalid-email".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_url() {
        let mut config = create_valid_config();
        config.remote.api_url = "not-a-url".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_project_id() {
        let mut config = create_valid_config();
        config.repo.project_id = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_from_str_valid() {
        let toml = r#"
[user]
name = "Test User"
email = "test@example.com"
auto_signoff = true

[remote]
provider = "Gitcode"
api_url = "https://api.example.com"
token = "token123"

[repo]
project_id = "owner/repo"
default_branch = "main"

[template]
pr_prefix = "[TEST]"
"#;
        let config = Config::parse_from_str(toml).unwrap();
        assert_eq!(config.user.name, "Test User");
        assert!(config.user.auto_signoff);
        assert_eq!(config.remote.provider, "Gitcode");
        assert_eq!(config.repo.default_branch, "main");
    }

    #[test]
    fn test_from_str_invalid_toml() {
        let toml = "invalid toml content";
        assert!(Config::parse_from_str(toml).is_err());
    }

    #[test]
    fn test_default_values() {
        let toml = r#"
[user]
name = "Test"
email = "test@example.com"

[remote]
provider = "Gitcode"
api_url = "https://api.example.com"
token = "token"

[repo]
project_id = "owner/repo"
"#;
        let config = Config::parse_from_str(toml).unwrap();
        assert_eq!(config.repo.default_branch, "main");
        assert!(!config.user.auto_signoff);
    }

    #[test]
    fn test_email_validation() {
        assert!(Config::is_valid_email("test@example.com"));
        assert!(Config::is_valid_email("user.name@domain.co"));
        assert!(!Config::is_valid_email("invalid"));
        assert!(!Config::is_valid_email("@example.com"));
        assert!(!Config::is_valid_email("test@"));
        assert!(!Config::is_valid_email("test"));
    }

    #[test]
    fn test_url_validation() {
        assert!(Config::is_valid_url("https://api.example.com"));
        assert!(Config::is_valid_url("http://localhost:8080"));
        assert!(!Config::is_valid_url("not-a-url"));
        assert!(!Config::is_valid_url("ftp://example.com"));
    }

    #[test]
    fn test_example_generation() {
        let example = Config::example();
        assert!(example.contains("[user]"));
        assert!(example.contains("[remote]"));
        assert!(example.contains("[repo]"));
        assert!(example.contains("[template]"));
    }

    #[test]
    fn test_ensure_config_dir() {
        let dir = Config::ensure_config_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.to_string_lossy().contains("vkt"));
    }

    #[test]
    fn test_provider_type_parse() {
        assert_eq!(ProviderType::parse("Gitcode"), ProviderType::GitCode);
        assert_eq!(ProviderType::parse("gitcode"), ProviderType::GitCode);
        assert_eq!(ProviderType::parse("GITCODE"), ProviderType::GitCode);
        assert_eq!(ProviderType::parse("GitLab"), ProviderType::GitLab);
        assert_eq!(ProviderType::parse("gitlab"), ProviderType::GitLab);
        assert_eq!(ProviderType::parse("GitHub"), ProviderType::GitHub);
        assert_eq!(ProviderType::parse("github"), ProviderType::GitHub);
        assert_eq!(
            ProviderType::parse("Unknown"),
            ProviderType::Unknown("unknown".to_string())
        );
    }

    #[test]
    fn test_provider_type_as_str() {
        assert_eq!(ProviderType::GitCode.as_str(), "gitcode");
        assert_eq!(ProviderType::GitLab.as_str(), "gitlab");
        assert_eq!(ProviderType::GitHub.as_str(), "github");
        assert_eq!(
            ProviderType::Unknown("custom".to_string()).as_str(),
            "custom"
        );
    }

    #[test]
    fn test_detect_provider() {
        assert_eq!(
            detect_provider("https://gitcode.com/api/v5"),
            ProviderType::GitCode
        );
        assert_eq!(
            detect_provider("https://gitlab.example.com/api/v4"),
            ProviderType::GitLab
        );
        assert_eq!(
            detect_provider("https://github.com/api/v3"),
            ProviderType::GitHub
        );
        assert_eq!(
            detect_provider("https://unknown.example.com/api"),
            ProviderType::Unknown("unknown".to_string())
        );
    }

    #[test]
    fn test_remote_config_provider_type() {
        let remote = RemoteConfig {
            provider: "Gitcode".to_string(),
            api_url: "https://gitcode.com/api/v5".to_string(),
            token: "test-token".to_string(),
        };
        assert_eq!(remote.provider_type(), ProviderType::GitCode);
    }
}
