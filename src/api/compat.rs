//! Backwards Compatibility Layer
//!
//! Provides API compatibility with the existing ApiClient interface
//! while using the new ForgeProvider trait internally.

use crate::config::Config;
use crate::error::Result;

use super::factory::create_provider;
use super::traits::ForgeProvider;
use super::types::{Branch, FileCommitResponse, FileInfo, PullRequest, TreeItem};

/// API Client (backwards compatible)
///
/// This struct maintains the same interface as the original ApiClient
/// but delegates to a ForgeProvider implementation internally.
pub struct ApiClient {
    provider: Box<dyn ForgeProvider>,
}

impl std::fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiClient")
            .field("provider", &"<dyn ForgeProvider>")
            .finish()
    }
}

impl ApiClient {
    /// Create a new API client
    ///
    /// # Arguments
    /// * `config` - The VKT configuration
    ///
    /// # Returns
    /// A new ApiClient instance
    pub fn new(config: &Config) -> Result<Self> {
        let provider = create_provider(config)?;
        Ok(Self { provider })
    }

    /// List repository contents
    ///
    /// # Arguments
    /// * `path` - Optional path to list (root if None)
    /// * `recursive` - Whether to list recursively
    /// * `ref_branch` - Optional branch/ref to list from
    ///
    /// # Returns
    /// List of tree items representing files and directories
    pub async fn list_repository_tree(
        &self,
        path: Option<&str>,
        recursive: bool,
        ref_branch: Option<&str>,
    ) -> Result<Vec<TreeItem>> {
        self.provider
            .list_repository_tree(path, recursive, ref_branch)
            .await
    }

    /// Get file information (metadata)
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `ref_branch` - Optional branch/ref to get from
    ///
    /// # Returns
    /// File information including size, SHA, etc.
    pub async fn get_file_info(
        &self,
        file_path: &str,
        ref_branch: Option<&str>,
    ) -> Result<FileInfo> {
        self.provider.get_file_info(file_path, ref_branch).await
    }

    /// Get file raw content
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `ref_branch` - Optional branch/ref to get from
    ///
    /// # Returns
    /// Raw file content as bytes
    pub async fn get_file_raw(&self, file_path: &str, ref_branch: Option<&str>) -> Result<Vec<u8>> {
        self.provider.get_file_content(file_path, ref_branch).await
    }

    /// Create a new branch
    ///
    /// # Arguments
    /// * `branch_name` - Name for the new branch
    /// * `source_branch` - Source branch to create from
    ///
    /// # Returns
    /// Information about the created branch
    pub async fn create_branch(&self, branch_name: &str, source_branch: &str) -> Result<Branch> {
        self.provider
            .create_branch(branch_name, source_branch)
            .await
    }

    /// Create or update a file
    ///
    /// # Arguments
    /// * `file_path` - Path where the file should be created/updated
    /// * `content` - Base64-encoded file content
    /// * `branch` - Branch to create/update the file on
    /// * `message` - Commit message
    /// * `author_name` - Author name for the commit
    /// * `author_email` - Author email for the commit
    ///
    /// # Returns
    /// Response containing commit and file information
    pub async fn create_or_update_file(
        &self,
        file_path: &str,
        content: &str,
        branch: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<FileCommitResponse> {
        self.provider
            .create_or_update_file(
                file_path,
                content,
                branch,
                message,
                author_name,
                author_email,
            )
            .await
    }

    /// Create a pull request
    ///
    /// # Arguments
    /// * `title` - PR title
    /// * `head_branch` - Source branch (containing changes)
    /// * `base_branch` - Target branch (to merge into)
    /// * `body` - Optional PR description
    ///
    /// # Returns
    /// Created pull request information
    pub async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: Option<&str>,
    ) -> Result<PullRequest> {
        self.provider
            .create_pull_request(title, head_branch, base_branch, body)
            .await
    }

    /// Assign reviewers to a pull request
    ///
    /// # Arguments
    /// * `pr_number` - Pull request number
    /// * `reviewers` - List of reviewer usernames
    pub async fn assign_reviewers(&self, pr_number: u64, reviewers: &[String]) -> Result<()> {
        self.provider.assign_reviewers(pr_number, reviewers).await
    }

    /// Get repository information
    ///
    /// # Returns
    /// Repository metadata
    pub async fn get_repository_info(&self) -> Result<super::types::RepositoryInfo> {
        self.provider.get_repository_info().await
    }
}

#[cfg(test)]
mod tests {}
