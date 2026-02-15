//! Git/Forge API Client Module
//!
//! Provides a provider-based abstraction for Git hosting APIs.
//! Supports GitCode, GitLab, GitHub, and other Git hosting providers.

// Module declarations
pub mod compat;
pub mod factory;
pub mod gitcode;
pub mod traits;
pub mod types;

// Re-export main types for convenience
pub use compat::ApiClient;
pub use factory::create_provider;
pub use traits::ForgeProvider;
pub use types::*;

// Re-export gitcode types
pub use gitcode::GitCodeProvider;

// Re-export factory function
pub use factory::detect_provider;

#[cfg(test)]
mod tests {
    use crate::config::{Config, RemoteConfig, RepoConfig, TemplateConfig, UserConfig};

    fn create_test_config(base_url: String) -> Config {
        Config {
            user: UserConfig {
                name: "Test".to_string(),
                email: "test@example.com".to_string(),
                auto_signoff: true,
            },
            remote: RemoteConfig {
                provider: "Gitcode".to_string(),
                api_url: base_url,
                token: "test-token".to_string(),
            },
            repo: RepoConfig {
                project_id: "owner/repo".to_string(),
                default_branch: "main".to_string(),
            },
            template: TemplateConfig::default(),
        }
    }

    #[test]
    fn test_api_client_new() {
        let config = create_test_config("https://api.example.com".to_string());
        let client = super::ApiClient::new(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_tree_item_is_dir() {
        let dir_item = super::types::TreeItem {
            id: "abc".to_string(),
            name: "src".to_string(),
            item_type: "tree".to_string(),
            path: "src".to_string(),
            mode: "040000".to_string(),
        };
        assert!(dir_item.is_dir());
        assert!(!dir_item.is_file());
    }

    #[test]
    fn test_tree_item_is_file() {
        let file_item = super::types::TreeItem {
            id: "def".to_string(),
            name: "main.rs".to_string(),
            item_type: "blob".to_string(),
            path: "src/main.rs".to_string(),
            mode: "100644".to_string(),
        };
        assert!(file_item.is_file());
        assert!(!file_item.is_dir());
    }

    #[test]
    fn test_tree_item_deserialization() {
        let json = r#"{
            "id": "abc123",
            "name": "README.md",
            "type": "blob",
            "path": "README.md",
            "mode": "100644"
        }"#;
        let item: super::types::TreeItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "README.md");
        assert!(item.is_file());
    }

    #[test]
    fn test_pull_request_deserialization() {
        let json = r#"{
            "number": 42,
            "title": "Test PR",
            "html_url": "https://example.com/pr/42",
            "state": "open"
        }"#;
        let pr: super::types::PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.html_url, Some("https://example.com/pr/42".to_string()));
    }

    #[test]
    fn test_file_info_deserialization() {
        let json = r#"{
            "file_name": "test.txt",
            "file_path": "test.txt",
            "size": 1024,
            "content": "SGVsbG8="
        }"#;
        let info: super::types::FileInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, Some("test.txt".to_string()));
        assert_eq!(info.size, Some(1024));
    }

    #[test]
    fn test_api_error_type() {
        use crate::error::VktError;
        let err = VktError::Api("test error".to_string());
        assert_eq!(format!("{}", err), "API error: test error");
    }

    #[test]
    fn test_forge_provider_trait_is_object_safe() {
        // This test ensures the trait is object-safe
        fn _assert_object_safe(_: &dyn super::ForgeProvider) {}
    }

    #[test]
    fn test_create_provider_gitcode() {
        let config = create_test_config("https://gitcode.com/api/v5".to_string());
        let provider = super::create_provider(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_provider_type_detection() {
        use super::factory::detect_provider;
        use crate::config::ProviderType;

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
    }
}
