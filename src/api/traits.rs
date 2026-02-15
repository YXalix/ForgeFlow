//! Forge Provider Trait
//!
//! Defines the abstract interface for all Git/Forge API operations.
//! This trait abstracts over different Git hosting providers (GitCode, GitLab, GitHub, etc.)

use async_trait::async_trait;

use crate::error::Result;

use super::types::{Branch, FileCommitResponse, FileInfo, PullRequest, RepositoryInfo, TreeItem};

/// Forge Provider trait
///
/// This trait defines the interface that all Git hosting providers must implement.
/// It provides methods for repository operations, file management, and pull request workflows.
#[async_trait]
pub trait ForgeProvider: Send + Sync {
    /// List repository contents
    ///
    /// # Arguments
    /// * `path` - Optional path to list (root if None)
    /// * `recursive` - Whether to list recursively
    /// * `ref_branch` - Optional branch/ref to list from
    ///
    /// # Returns
    /// List of tree items representing files and directories
    async fn list_repository_tree(
        &self,
        path: Option<&str>,
        recursive: bool,
        ref_branch: Option<&str>,
    ) -> Result<Vec<TreeItem>>;

    /// Get file content as raw bytes
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `ref_branch` - Optional branch/ref to get from
    ///
    /// # Returns
    /// Raw file content as bytes
    async fn get_file_content(&self, file_path: &str, ref_branch: Option<&str>) -> Result<Vec<u8>>;

    /// Get file metadata
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `ref_branch` - Optional branch/ref to get from
    ///
    /// # Returns
    /// File information including size, SHA, etc.
    async fn get_file_info(&self, file_path: &str, ref_branch: Option<&str>) -> Result<FileInfo>;

    /// Create a new branch
    ///
    /// # Arguments
    /// * `branch_name` - Name for the new branch
    /// * `source_branch` - Source branch to create from
    ///
    /// # Returns
    /// Information about the created branch
    async fn create_branch(&self, branch_name: &str, source_branch: &str) -> Result<Branch>;

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
    async fn create_or_update_file(
        &self,
        file_path: &str,
        content: &str,
        branch: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<FileCommitResponse>;

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
    async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: Option<&str>,
    ) -> Result<PullRequest>;

    /// Assign reviewers to a pull request
    ///
    /// # Arguments
    /// * `pr_number` - Pull request number
    /// * `reviewers` - List of reviewer usernames
    async fn assign_reviewers(&self, pr_number: u64, reviewers: &[String]) -> Result<()>;

    /// Get repository information
    ///
    /// # Returns
    /// Repository metadata
    async fn get_repository_info(&self) -> Result<RepositoryInfo>;

    /// Check if a file exists
    ///
    /// # Arguments
    /// * `file_path` - Path to check
    /// * `ref_branch` - Optional branch/ref to check on
    ///
    /// # Returns
    /// true if the file exists, false otherwise
    ///
    /// # Default Implementation
    /// The default implementation tries to get file info and returns true if successful.
    async fn file_exists(&self, file_path: &str, ref_branch: Option<&str>) -> Result<bool> {
        match self.get_file_info(file_path, ref_branch).await {
            Ok(_) => Ok(true),
            Err(e) if e.is_not_found() => Ok(false),
            Err(e) => Err(e),
        }
    }
}
