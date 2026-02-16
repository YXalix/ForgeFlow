//! GitCode Provider Implementation
//!
//! Implements the ForgeProvider trait for GitCode's GitHub-compatible API.

use async_trait::async_trait;
use reqwest::{Client, Method, Response, StatusCode};

use crate::api::traits::ForgeProvider;
use crate::api::types::{
    Branch, Commit, FileCommitResponse, FileInfo, PullRequest, RepositoryInfo, TreeItem,
};
use crate::config::Config;
use crate::error::{Result, VktError};

mod types;

use types::{
    CreateBranchRequest, CreateFileRequest, CreatePullRequest, GitCodeBranchResponse,
    GitCodeCommitResponse, GitCodeContentResponse, GitCodePullResponse, GitCodeRepoResponse, GitTreeResponse, RequestReviewers, path_to_tree_item,
};

/// GitCode API Provider
#[derive(Debug, Clone)]
pub struct GitCodeProvider {
    /// HTTP client
    client: Client,
    /// Base API URL
    base_url: String,
    /// API token
    token: String,
    /// Repository owner
    owner: String,
    /// Repository name
    repo: String,
    /// Default branch name
    default_branch: String,
}

impl GitCodeProvider {
    /// Create a new GitCode provider from configuration
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| VktError::Network(e.to_string()))?;

        let (owner, repo) = Self::parse_project_id(&config.repo.project_id)?;

        Ok(Self {
            client,
            base_url: config.remote.api_url.trim_end_matches('/').to_string(),
            token: config.remote.token.clone(),
            owner,
            repo,
            default_branch: config.repo.default_branch.clone(),
        })
    }

    /// Parse project ID in "owner/repo" format
    fn parse_project_id(project_id: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = project_id.split('/').collect();
        if parts.len() != 2 {
            return Err(VktError::Config(
                "Project ID format should be 'owner/repo'".to_string(),
            ));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Build full API URL
    fn build_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// Build authenticated request
    fn build_request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = self.build_url(path);
        self.client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "vkt/0.1.0")
    }

    /// Handle API response with proper error mapping
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T> {
        let status = response.status();
        if status.is_success() {
            // Get the response text first for better error messages
            let text = response
                .text()
                .await
                .map_err(|e| VktError::Api(format!("Failed to read response body: {}", e)))?;

            // Parse JSON with detailed error message
            serde_json::from_str(&text).map_err(|e| {
                VktError::Api(format!(
                    "Failed to parse response: {}. Body: {}",
                    e,
                    if text.len() > 200 {
                        format!("{}...", &text[..200])
                    } else {
                        text
                    }
                ))
            })
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(match status {
                StatusCode::UNAUTHORIZED => {
                    VktError::AuthInvalid(format!("Authentication failed: {}", error_text))
                }
                StatusCode::FORBIDDEN => {
                    if error_text.to_lowercase().contains("rate")
                        || error_text.to_lowercase().contains("limit")
                    {
                        VktError::RateLimited(format!("Rate limited: {}", error_text))
                    } else {
                        VktError::PermissionDenied(format!("Permission denied: {}", error_text))
                    }
                }
                StatusCode::NOT_FOUND => {
                    VktError::ApiNotFound(format!("Resource not found: {}", error_text))
                }
                StatusCode::CONFLICT => {
                    VktError::Conflict(format!("Resource conflict: {}", error_text))
                }
                _ => VktError::Api(format!("API error (HTTP {}): {}", status, error_text)),
            })
        }
    }

    /// Process API paths into TreeItems
    /// - Filters by parent path if specified
    /// - For non-recursive: returns only immediate children
    /// - For recursive: returns all items under the path
    fn process_paths(
        &self,
        paths: Vec<String>,
        parent_path: Option<&str>,
        recursive: bool,
    ) -> Vec<TreeItem> {
        use sha2::{Digest, Sha256};
        use std::collections::HashMap;

        // Normalize the prefix: remove leading/trailing slashes
        let prefix = parent_path.map(|p| p.trim_matches('/')).unwrap_or("");
        let prefix_with_slash = if prefix.is_empty() {
            "".to_string()
        } else {
            format!("{}/", prefix)
        };

        // Filter paths under the parent directory
        let filtered: Vec<String> = paths
            .into_iter()
            .filter(|p| {
                let p = p.trim_end_matches('/');
                if prefix.is_empty() {
                    true
                } else {
                    p == prefix || p.starts_with(&prefix_with_slash)
                }
            })
            .collect();

        if recursive {
            // Return all items with full paths
            filtered.into_iter().map(path_to_tree_item).collect()
        } else {
            // Extract only immediate children
            let mut children: HashMap<String, TreeItem> = HashMap::new();

            for path in filtered {
                let relative_path = if prefix.is_empty() {
                    path.clone()
                } else {
                    path.strip_prefix(&prefix_with_slash)
                        .map(|s| s.to_string())
                        .unwrap_or(path.clone())
                };

                // Get the first component (immediate child)
                let trimmed = relative_path.trim_end_matches('/');
                let first_sep = trimmed.find('/');
                let (name, is_dir) = match first_sep {
                    Some(pos) => {
                        // There's a nested component, so this is a directory
                        (trimmed[..pos].to_string(), true)
                    }
                    None => {
                        // No nested component - check if original path ends with /
                        // to determine if it's a directory
                        (trimmed.to_string(), path.ends_with('/'))
                    }
                };

                // Use the full reconstructed path for the child
                let full_child_path = if prefix.is_empty() {
                    if is_dir {
                        format!("{}/", name)
                    } else {
                        name.clone()
                    }
                } else {
                    format!("{}/{}", prefix, name)
                };

                // Insert if not already present (deduplication)
                if !children.contains_key(&name) {
                    let id = format!("{:x}", Sha256::digest(full_child_path.as_bytes()))[..16]
                        .to_string();
                    children.insert(
                        name.clone(),
                        TreeItem {
                            id,
                            name,
                            item_type: if is_dir {
                                "tree".to_string()
                            } else {
                                "blob".to_string()
                            },
                            path: full_child_path.trim_end_matches('/').to_string(),
                            mode: if is_dir {
                                "040000".to_string()
                            } else {
                                "100644".to_string()
                            },
                        },
                    );
                }
            }

            children.into_values().collect()
        }
    }
}

#[async_trait]
impl ForgeProvider for GitCodeProvider {
    /// List repository tree items
    /// https://api.gitcode.com/api/v5/repos/:owner/:repo/file_list
    ///
    /// Returns a simple array of file paths like:
    /// [".assets/公开开源.jpg", ".assets/导入题目.jpg", "src/main.rs", "docs/"]
    ///
    /// Query Parameters:
    ///     access_token: API token for authentication
    ///     ref_name: ref (branch, tag, commit)
    ///     file_name: The name of the file to be searched for.
    async fn list_repository_tree(
        &self,
        path: Option<&str>,
        recursive: bool,
        ref_branch: Option<&str>,
    ) -> Result<Vec<TreeItem>> {
        let mut api_path = format!("repos/{}/{}/file_list", self.owner, self.repo);

        // Build query parameters
        let mut params = Vec::new();

        if let Some(ref_branch) = ref_branch {
            params.push(format!("ref_name={}", urlencoding::encode(ref_branch)));
        }

        if !params.is_empty() {
            api_path = format!("{}?{}", api_path, params.join("&"));
        }

        let response = self.build_request(Method::GET, &api_path).send().await?;
        let paths: GitTreeResponse = self.handle_response(response).await?;

        // Process paths to extract immediate children (ls-like behavior)
        let items = self.process_paths(paths, path, recursive);

        Ok(items)
    }

    async fn get_file_content(&self, file_path: &str, ref_branch: Option<&str>) -> Result<Vec<u8>> {
        let encoded_path = urlencoding::encode(file_path);
        let mut api_path = format!(
            "repos/{}/{}/contents/{}",
            self.owner, self.repo, encoded_path
        );

        if let Some(branch) = ref_branch {
            api_path = format!("{}?ref={}", api_path, urlencoding::encode(branch));
        }

        let response = self.build_request(Method::GET, &api_path).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(VktError::ApiNotFound(format!(
                "File not found: {}",
                file_path
            )));
        }

        let content_response: GitCodeContentResponse = self.handle_response(response).await?;

        // Decode base64 content
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(&content_response.content)
            .map_err(|e| VktError::Api(format!("Failed to decode file content: {}", e)))
    }

    async fn get_file_info(&self, file_path: &str, ref_branch: Option<&str>) -> Result<FileInfo> {
        let encoded_path = urlencoding::encode(file_path);
        let mut api_path = format!(
            "repos/{}/{}/contents/{}",
            self.owner, self.repo, encoded_path
        );

        if let Some(branch) = ref_branch {
            api_path = format!("{}?ref={}", api_path, urlencoding::encode(branch));
        }

        let response = self.build_request(Method::GET, &api_path).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(VktError::ApiNotFound(format!(
                "File not found: {}",
                file_path
            )));
        }

        let content_response: GitCodeContentResponse = self.handle_response(response).await?;
        Ok(content_response.into())
    }

    async fn create_branch(&self, branch_name: &str, source_branch: &str) -> Result<Branch> {
        // GitCode/Gitee API: POST /repos/{owner}/{repo}/branches
        let path = format!("repos/{}/{}/branches", self.owner, self.repo);
        let body = CreateBranchRequest {
            branch_name: branch_name.to_string(),
            refs: source_branch.to_string(),
        };

        let response = self
            .build_request(Method::POST, &path)
            .json(&body)
            .send()
            .await?;

        let branch_info: GitCodeBranchResponse = self.handle_response(response).await?;

        // Extract commit info from nested structure
        let commit_sha = branch_info
            .commit
            .sha
            .or_else(|| branch_info.commit.id.clone())
            .or_else(|| {
                branch_info
                    .commit
                    .commit
                    .as_ref()
                    .and_then(|c| c.sha.clone())
            })
            .ok_or_else(|| {
                VktError::Api("Could not extract commit SHA from branch response".to_string())
            })?;

        let commit_message = branch_info
            .commit
            .message
            .or_else(|| {
                branch_info
                    .commit
                    .commit
                    .as_ref()
                    .and_then(|c| c.message.clone())
            })
            .unwrap_or_default();

        let authored_date = branch_info
            .commit
            .commit
            .as_ref()
            .and_then(|c| c.authored_date.clone());

        Ok(Branch {
            name: branch_info.name,
            commit: Commit {
                id: commit_sha,
                message: commit_message,
                author: None,
                timestamp: authored_date,
            },
        })
    }

    async fn create_or_update_file(
        &self,
        file_path: &str,
        content: &str,
        branch: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<FileCommitResponse> {
        let encoded_path = urlencoding::encode(file_path);
        let path = format!(
            "repos/{}/{}/contents/{}",
            self.owner, self.repo, encoded_path
        );

        // Try to get the existing file SHA (needed for updates)
        // First try target branch, then default branch
        let existing_sha = match self.get_file_info(file_path, Some(branch)).await {
            Ok(file_info) => {
                println!("📋 File exists on target branch, SHA: {:?}", file_info.sha);
                file_info.sha
            }
            Err(_) => {
                // File doesn't exist on target branch, check default branch
                match self
                    .get_file_info(file_path, Some(&self.default_branch))
                    .await
                {
                    Ok(file_info) => {
                        println!("📋 File exists on default branch, SHA: {:?}", file_info.sha);
                        file_info.sha
                    }
                    Err(_) => {
                        println!("📋 File doesn't exist on any branch, creating new file");
                        None
                    }
                }
            }
        };

        let body = CreateFileRequest {
            message: message.to_string(),
            content: content.to_string(),
            branch: branch.to_string(),
            sha: existing_sha.clone(),
            author_name: Some(author_name.to_string()),
            author_email: Some(author_email.to_string()),
        };

        // Use POST for new files, PUT for updates
        let method = if existing_sha.is_some() {
            println!("📤 Updating file with PUT (SHA: {:?})", existing_sha);
            Method::PUT
        } else {
            println!("📤 Creating file with POST");
            Method::POST
        };

        let response = self.build_request(method, &path).json(&body).send().await?;

        let commit_response: GitCodeCommitResponse = self.handle_response(response).await?;
        Ok(commit_response.into())
    }

    async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: Option<&str>,
    ) -> Result<PullRequest> {
        let path = format!("repos/{}/{}/pulls", self.owner, self.repo);
        let body = CreatePullRequest {
            title: title.to_string(),
            head: head_branch.to_string(),
            base: base_branch.to_string(),
            body: body.map(|s| s.to_string()),
        };

        let response = self
            .build_request(Method::POST, &path)
            .json(&body)
            .send()
            .await?;

        let pull_response: GitCodePullResponse = self.handle_response(response).await?;
        Ok(pull_response.into())
    }

    async fn assign_reviewers(&self, pr_number: u64, reviewers: &[String]) -> Result<()> {
        // GitCode/Gitee uses different endpoints for reviewer assignment
        // Try both GitHub-style and GitLab-style endpoints

        // Try GitHub-style first: POST /repos/{owner}/{repo}/pulls/{pr_number}/requested_reviewers
        let github_path = format!(
            "repos/{}/{}/pulls/{}/requested_reviewers",
            self.owner, self.repo, pr_number
        );
        let body = RequestReviewers {
            reviewers: reviewers.to_vec(),
        };

        let response = self
            .build_request(Method::POST, &github_path)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            let _: GitCodePullResponse = self.handle_response(response).await?;
            return Ok(());
        }

        // If GitHub-style fails, this might need GitLab-style API
        // For now, treat reviewer assignment as optional - PR is already created
        println!("⚠️  Reviewer assignment endpoint not supported by this GitCode instance");
        println!("   You can manually assign reviewers via the web interface");
        Ok(())
    }

    async fn get_repository_info(&self) -> Result<RepositoryInfo> {
        let path = format!("repos/{}/{}", self.owner, self.repo);
        let response = self.build_request(Method::GET, &path).send().await?;
        let repo_response: GitCodeRepoResponse = self.handle_response(response).await?;
        Ok(repo_response.into())
    }

    /// Check if a file exists using the file_list API with file_name parameter
    /// https://api.gitcode.com/api/v5/repos/:owner/:repo/file_list
    async fn file_exists(&self, file_path: &str, ref_branch: Option<&str>) -> Result<bool> {
        let mut api_path = format!("repos/{}/{}/file_list", self.owner, self.repo);

        // Build query parameters with file_name to search for specific file
        let mut params = Vec::new();

        // Add file_name parameter to search for the specific file
        params.push(format!("file_name={}", urlencoding::encode(file_path)));

        if let Some(ref_branch) = ref_branch {
            params.push(format!("ref_name={}", urlencoding::encode(ref_branch)));
        }

        if !params.is_empty() {
            api_path = format!("{}?{}", api_path, params.join("&"));
        }

        let response = self.build_request(Method::GET, &api_path).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }

        let paths: GitTreeResponse = self.handle_response(response).await?;

        // Check if the file path is in the returned list
        Ok(paths.iter().any(|p| p.trim_end_matches('/') == file_path.trim_end_matches('/')))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider() -> GitCodeProvider {
        // Create a minimal provider for testing process_paths logic
        GitCodeProvider {
            client: Client::new(),
            base_url: "https://api.gitcode.com/api/v5".to_string(),
            token: "test".to_string(),
            owner: "test".to_string(),
            repo: "test".to_string(),
            default_branch: "main".to_string(),
        }
    }

    #[test]
    fn test_process_paths_with_leading_slash() {
        let provider = create_test_provider();
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/commands/mod.rs".to_string(),
            "Cargo.toml".to_string(),
        ];

        // Test with leading slash - should normalize and work
        let items = provider.process_paths(paths.clone(), Some("/src"), false);
        assert_eq!(items.len(), 3);

        let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
        assert!(names.contains(&"main.rs".to_string()));
        assert!(names.contains(&"lib.rs".to_string()));
        assert!(names.contains(&"commands".to_string()));
    }

    #[test]
    fn test_process_paths_file_not_treated_as_dir() {
        let provider = create_test_provider();
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/commands/mod.rs".to_string(),
        ];

        let items = provider.process_paths(paths, Some("src"), false);

        // main.rs should be a file, not a directory
        let main_rs = items.iter().find(|i| i.name == "main.rs").unwrap();
        assert!(!main_rs.is_dir());
        assert_eq!(main_rs.item_type, "blob");

        // commands should be a directory
        let commands = items.iter().find(|i| i.name == "commands").unwrap();
        assert!(commands.is_dir());
        assert_eq!(commands.item_type, "tree");
    }

    #[test]
    fn test_process_paths_root_listing() {
        let provider = create_test_provider();
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ];

        let items = provider.process_paths(paths, None, false);
        assert_eq!(items.len(), 3); // src/, Cargo.toml, README.md

        let names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
        assert!(names.contains(&"src".to_string()));
        assert!(names.contains(&"Cargo.toml".to_string()));
        assert!(names.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_process_paths_recursive() {
        let provider = create_test_provider();
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
        ];

        let items = provider.process_paths(paths.clone(), None, true);
        assert_eq!(items.len(), 3);
    }
}
