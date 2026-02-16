//! Configuration management module
//!
//! Handles loading and validation of TOML configuration files

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    /// Get the default API URL for this provider
    pub fn default_api_url(&self) -> Option<&'static str> {
        match self {
            ProviderType::GitCode => Some("https://api.gitcode.com/api/v5"),
            ProviderType::GitLab => Some("https://gitlab.com/api/v4"),
            ProviderType::GitHub => Some("https://api.github.com"),
            ProviderType::Unknown(_) => None,
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

    /// Parse dotted key path (e.g., "user.name") into section and field
    pub fn parse_key(key: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 2 {
            return Err(VktError::Validation(format!(
                "Invalid key format '{}'. Use format: section.field (e.g., user.name)",
                key
            )));
        }
        Ok((parts[0], parts[1]))
    }

    /// Get a config value as string by key path
    /// Note: remote.token is masked for security; use environment variable for scripting
    pub fn get_value(&self, key: &str) -> Result<String> {
        let (section, field) = Self::parse_key(key)?;

        match (section, field) {
            ("user", "name") => Ok(self.user.name.clone()),
            ("user", "email") => Ok(self.user.email.clone()),
            ("user", "auto_signoff") => Ok(self.user.auto_signoff.to_string()),
            ("remote", "provider") => Ok(self.remote.provider.clone()),
            ("remote", "api_url") => Ok(self.remote.api_url.clone()),
            ("remote", "token") => Ok("********".to_string()),
            ("repo", "project_id") => Ok(self.repo.project_id.clone()),
            ("repo", "default_branch") => Ok(self.repo.default_branch.clone()),
            ("template", "pr_prefix") => Ok(self.template.pr_prefix.clone()),
            _ => Err(VktError::Validation(format!("Unknown config key: {}", key))),
        }
    }

    /// Update a single config value by key path
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        let (section, field) = Self::parse_key(key)?;

        match (section, field) {
            ("user", "name") => {
                if value.is_empty() {
                    return Err(VktError::Validation("User name cannot be empty".to_string()));
                }
                self.user.name = value.to_string();
            }
            ("user", "email") => {
                if !Self::is_valid_email(value) {
                    return Err(VktError::Validation(format!("Invalid email: {}", value)));
                }
                self.user.email = value.to_string();
            }
            ("user", "auto_signoff") => {
                self.user.auto_signoff = value.parse().map_err(|_| {
                    VktError::Validation(format!("Expected boolean value: {}", value))
                })?;
            }
            ("remote", "provider") => {
                if value.is_empty() {
                    return Err(VktError::Validation("Provider cannot be empty".to_string()));
                }
                self.remote.provider = value.to_string();
            }
            ("remote", "api_url") => {
                if !Self::is_valid_url(value) {
                    return Err(VktError::Validation(format!("Invalid URL: {}", value)));
                }
                self.remote.api_url = value.to_string();
            }
            ("remote", "token") => {
                if value.is_empty() {
                    return Err(VktError::Validation("Token cannot be empty".to_string()));
                }
                self.remote.token = value.to_string();
            }
            ("repo", "project_id") => {
                if !value.contains('/') {
                    return Err(VktError::Validation(
                        "Project ID must be in format: owner/repo".to_string(),
                    ));
                }
                self.repo.project_id = value.to_string();
            }
            ("repo", "default_branch") => {
                if value.is_empty() {
                    return Err(VktError::Validation(
                        "Default branch cannot be empty".to_string(),
                    ));
                }
                self.repo.default_branch = value.to_string();
            }
            ("template", "pr_prefix") => {
                self.template.pr_prefix = value.to_string();
            }
            _ => return Err(VktError::Validation(format!("Unknown config key: {}", key))),
        }
        Ok(())
    }

    /// Save config to file atomically
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Serialize to TOML
        let content = toml::to_string_pretty(self)
            .map_err(|e| VktError::Config(format!("Failed to serialize config: {}", e)))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(VktError::Io)?;
            }
        }

        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, content).map_err(VktError::Io)?;

        // Atomic rename
        std::fs::rename(&temp_path, path).map_err(VktError::Io)?;

        Ok(())
    }

    /// Run interactive configuration setup
    pub fn interactive_setup() -> Result<Self> {
        use std::io::{self, Write};

        println!("Welcome to ForgeFlow! Let's set up your configuration.\n");

        /// Prompt for user input with a message
        fn prompt(message: &str) -> Result<String> {
            print!("{}", message);
            io::stdout().flush().map_err(VktError::Io)?;
            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(VktError::Io)?;
            Ok(input.trim().to_string())
        }

        /// Prompt with a default value if user enters nothing
        fn prompt_with_default(message: &str, default: &str) -> Result<String> {
            print!("{} [{}]: ", message, default);
            io::stdout().flush().map_err(VktError::Io)?;
            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(VktError::Io)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                Ok(default.to_string())
            } else {
                Ok(trimmed.to_string())
            }
        }

        /// Prompt for a boolean value (y/n)
        fn prompt_bool(message: &str) -> Result<bool> {
            loop {
                print!("{}", message);
                io::stdout().flush().map_err(VktError::Io)?;
                let mut input = String::new();
                io::stdin().read_line(&mut input).map_err(VktError::Io)?;
                match input.trim().to_lowercase().as_str() {
                    "y" | "yes" | "true" | "1" => return Ok(true),
                    "n" | "no" | "false" | "0" => return Ok(false),
                    _ => println!("Please enter 'y' or 'n'"),
                }
            }
        }

        // User section
        let name = loop {
            let name = prompt("Your name: ")?;
            if !name.is_empty() {
                break name;
            }
            println!("Name cannot be empty. Please try again.");
        };

        let email = loop {
            let email = prompt("Your email: ")?;
            if Self::is_valid_email(&email) {
                break email;
            }
            println!("Invalid email format. Please try again.");
        };

        let auto_signoff = prompt_bool("Auto sign-off commits? (y/n): ")?;

        // Remote section
        let provider = prompt_with_default("Provider (Gitcode/GitLab/GitHub)", "Gitcode")?;

        // Get default API URL based on provider
        let provider_type = ProviderType::parse(&provider);
        let default_url = provider_type
            .default_api_url()
            .unwrap_or("https://api.example.com");

        let api_url = loop {
            let url = prompt_with_default("API URL", default_url)?;
            if Self::is_valid_url(&url) {
                break url;
            }
            println!("Invalid URL format. Please try again.");
        };

        let token = loop {
            let token = prompt("API Token: ")?;
            if !token.is_empty() {
                break token;
            }
            println!("Token cannot be empty. Please try again.");
        };

        // Repo section
        let project_id = loop {
            let id = prompt("Project ID (owner/repo): ")?;
            if id.contains('/') {
                break id;
            }
            println!("Project ID must be in format: owner/repo");
        };

        let default_branch = prompt_with_default("Default branch", "main")?;

        // Template section
        let pr_prefix = prompt_with_default("PR prefix", "[VIRT-TOOL]")?;

        let config = Config {
            user: UserConfig {
                name,
                email,
                auto_signoff,
            },
            remote: RemoteConfig {
                provider,
                api_url,
                token,
            },
            repo: RepoConfig {
                project_id,
                default_branch,
            },
            template: TemplateConfig { pr_prefix },
        };

        // Validate before returning
        config.validate()?;

        Ok(config)
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
api_url = "https://api.gitcode.com/api/v5"
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
            api_url: "https://api.gitcode.com".to_string(),
            token: "test-token".to_string(),
        };
        assert_eq!(remote.provider_type(), ProviderType::GitCode);
    }

    #[test]
    fn test_provider_type_default_api_url() {
        assert_eq!(
            ProviderType::GitCode.default_api_url(),
            Some("https://api.gitcode.com/api/v5")
        );
        assert_eq!(
            ProviderType::GitLab.default_api_url(),
            Some("https://gitlab.com/api/v4")
        );
        assert_eq!(
            ProviderType::GitHub.default_api_url(),
            Some("https://api.github.com")
        );
        assert_eq!(
            ProviderType::Unknown("custom".to_string()).default_api_url(),
            None
        );
    }

    #[test]
    fn test_parse_key_valid() {
        let (section, field) = Config::parse_key("user.name").unwrap();
        assert_eq!(section, "user");
        assert_eq!(field, "name");

        let (section, field) = Config::parse_key("remote.token").unwrap();
        assert_eq!(section, "remote");
        assert_eq!(field, "token");
    }

    #[test]
    fn test_parse_key_invalid() {
        assert!(Config::parse_key("name").is_err());
        assert!(Config::parse_key("user.name.extra").is_err());
        assert!(Config::parse_key("").is_err());
    }

    #[test]
    fn test_get_value() {
        let config = create_valid_config();
        assert_eq!(config.get_value("user.name").unwrap(), "Test");
        assert_eq!(config.get_value("user.email").unwrap(), "test@example.com");
        assert_eq!(config.get_value("user.auto_signoff").unwrap(), "true");
        assert_eq!(config.get_value("remote.provider").unwrap(), "Gitcode");
        assert_eq!(config.get_value("remote.api_url").unwrap(), "https://api.example.com");
        // Token is masked for security
        assert_eq!(config.get_value("remote.token").unwrap(), "********");
        assert_eq!(config.get_value("repo.project_id").unwrap(), "owner/repo");
        assert_eq!(config.get_value("repo.default_branch").unwrap(), "main");
        assert_eq!(config.get_value("template.pr_prefix").unwrap(), "");
    }

    #[test]
    fn test_get_value_unknown_key() {
        let config = create_valid_config();
        assert!(config.get_value("unknown.key").is_err());
        assert!(config.get_value("user.unknown").is_err());
    }

    #[test]
    fn test_set_value_user_name() {
        let mut config = create_valid_config();
        config.set_value("user.name", "New Name").unwrap();
        assert_eq!(config.user.name, "New Name");
    }

    #[test]
    fn test_set_value_user_email() {
        let mut config = create_valid_config();
        config.set_value("user.email", "new@example.com").unwrap();
        assert_eq!(config.user.email, "new@example.com");
    }

    #[test]
    fn test_set_value_user_email_invalid() {
        let mut config = create_valid_config();
        assert!(config.set_value("user.email", "invalid-email").is_err());
    }

    #[test]
    fn test_set_value_user_auto_signoff() {
        let mut config = create_valid_config();
        config.set_value("user.auto_signoff", "false").unwrap();
        assert!(!config.user.auto_signoff);
        config.set_value("user.auto_signoff", "true").unwrap();
        assert!(config.user.auto_signoff);
    }

    #[test]
    fn test_set_value_remote_api_url() {
        let mut config = create_valid_config();
        config.set_value("remote.api_url", "https://new.example.com").unwrap();
        assert_eq!(config.remote.api_url, "https://new.example.com");
    }

    #[test]
    fn test_set_value_remote_api_url_invalid() {
        let mut config = create_valid_config();
        assert!(config.set_value("remote.api_url", "not-a-url").is_err());
    }

    #[test]
    fn test_set_value_repo_project_id() {
        let mut config = create_valid_config();
        config.set_value("repo.project_id", "newowner/newrepo").unwrap();
        assert_eq!(config.repo.project_id, "newowner/newrepo");
    }

    #[test]
    fn test_set_value_repo_project_id_invalid() {
        let mut config = create_valid_config();
        assert!(config.set_value("repo.project_id", "invalid").is_err());
    }

    #[test]
    fn test_set_value_unknown_key() {
        let mut config = create_valid_config();
        assert!(config.set_value("unknown.key", "value").is_err());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        use std::fs;

        let config = create_valid_config();
        let temp_path = std::env::temp_dir().join("vkt_test_config.toml");

        // Save config
        config.save_to_file(&temp_path).unwrap();

        // Load config
        let loaded = Config::parse_from_file(&temp_path).unwrap();
        assert_eq!(loaded.user.name, config.user.name);
        assert_eq!(loaded.user.email, config.user.email);
        assert_eq!(loaded.remote.provider, config.remote.provider);
        assert_eq!(loaded.repo.project_id, config.repo.project_id);

        // Cleanup
        fs::remove_file(&temp_path).unwrap();
    }
}
