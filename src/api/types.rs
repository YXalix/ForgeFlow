//! Shared API Types
//!
//! Common types used across all Git/Forge API providers.

use serde::{Deserialize, Serialize};

/// Repository tree entry
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TreeItem {
    /// Item ID (blob or tree SHA)
    pub id: String,
    /// Item name
    pub name: String,
    /// Item type ("blob" for file, "tree" for directory)
    #[serde(rename = "type")]
    pub item_type: String,
    /// Full path
    pub path: String,
    /// File mode (e.g., "100644" for regular file)
    pub mode: String,
}

impl TreeItem {
    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.item_type == "tree" || self.mode.starts_with('4')
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.item_type == "blob" || self.mode.starts_with("100")
    }
}

/// Branch information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Branch {
    /// Branch name
    pub name: String,
    /// Latest commit on this branch
    pub commit: Commit,
}

/// Commit metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commit {
    /// Commit SHA
    pub id: String,
    /// Commit message
    pub message: String,
    /// Optional: Author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    /// Optional: Commit timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Author/Committer information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Author {
    /// Author name
    pub name: String,
    /// Author email
    pub email: String,
    /// Optional: Commit date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// File metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileInfo {
    /// File name
    #[serde(rename = "file_name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// File path
    #[serde(rename = "file_path", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    /// File content (Base64 encoded, if requested)
    pub content: Option<String>,
    /// File SHA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

/// File content metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileContent {
    /// File name
    pub name: String,
    /// File path
    pub path: String,
    /// File SHA
    pub sha: String,
    /// File size in bytes
    pub size: Option<u64>,
    /// Download URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
}

/// File creation/update response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileCommitResponse {
    /// File content information
    pub content: FileContent,
    /// Commit information
    pub commit: Commit,
}

/// Pull Request information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequest {
    /// PR number
    pub number: u64,
    /// PR title
    pub title: String,
    /// PR URL (web)
    #[serde(rename = "html_url", skip_serializing_if = "Option::is_none")]
    pub html_url: Option<String>,
    /// PR state (open, closed, merged)
    pub state: String,
    /// Source branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<PullRequestRef>,
    /// Target branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<PullRequestRef>,
    /// PR body/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Pull Request branch reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PullRequestRef {
    /// Branch name
    pub ref_branch: String,
    /// Repository reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<RepositoryRef>,
}

/// Repository reference (for cross-repo PRs)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositoryRef {
    /// Repository full name (owner/repo)
    pub full_name: String,
}

/// Repository metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositoryInfo {
    /// Repository ID
    pub id: u64,
    /// Repository full name (owner/repo)
    pub full_name: String,
    /// Repository description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Default branch
    pub default_branch: String,
    /// Is repository private
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
    /// Repository web URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_url: Option<String>,
    /// Clone URL (HTTPS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clone_url: Option<String>,
    /// Clone URL (SSH)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_url: Option<String>,
}

/// Create branch request
#[derive(Debug, Clone, Serialize)]
pub struct CreateBranchRequest {
    /// New branch name
    #[serde(rename = "branch_name")]
    pub branch_name: String,
    /// Source ref (branch or commit SHA)
    #[serde(rename = "ref")]
    pub ref_branch: String,
}

/// Create or update file request
#[derive(Debug, Clone, Serialize)]
pub struct CreateOrUpdateFileRequest {
    /// Commit message
    pub message: String,
    /// Base64-encoded content
    pub content: String,
    /// Target branch
    pub branch: String,
    /// Optional: Committer info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committer: Option<CommitterInfo>,
    /// Optional: File SHA (for updates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
}

/// Committer information
#[derive(Debug, Clone, Serialize)]
pub struct CommitterInfo {
    /// Committer name
    pub name: String,
    /// Committer email
    pub email: String,
}

/// Create Pull Request request
#[derive(Debug, Clone, Serialize)]
pub struct CreatePullRequestRequest {
    /// PR title
    pub title: String,
    /// Source branch
    pub head: String,
    /// Target branch
    pub base: String,
    /// Optional: PR body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Assign reviewers request
#[derive(Debug, Clone, Serialize)]
pub struct AssignReviewersRequest {
    /// List of reviewer usernames
    pub reviewers: Vec<String>,
}

#[cfg(test)]
mod tests {}
