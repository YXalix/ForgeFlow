//! GitCode API Response Types
//!
//! These types map to GitCode's GitHub-compatible API responses.
//! GitCode uses a GitHub-style API, so these types closely mirror GitHub's API schema.

use serde::{Deserialize, Serialize};

use crate::api::types::{
    Author, Branch, Commit, FileCommitResponse, FileContent, FileInfo, PullRequest, PullRequestRef,
    RepositoryInfo, RepositoryRef, TreeItem,
};

/// Git tree listing response - file_list API returns a simple array of paths
pub type GitTreeResponse = Vec<String>;

/// Individual tree entry from GitCode API
#[derive(Debug, Clone, Deserialize)]
pub struct GitTreeItem {
    /// Item path
    pub path: String,
    /// Item mode (e.g., "100644", "040000")
    pub mode: String,
    /// Item type ("blob" for file, "tree" for directory, "commit" for submodule)
    #[serde(rename = "type")]
    pub item_type: String,
    /// Object SHA
    pub sha: String,
    /// Size in bytes (only for blobs)
    pub size: Option<u64>,
    /// API URL (optional, not always returned by GitCode)
    #[serde(default)]
    pub url: Option<String>,
}

/// Convert a file path string to TreeItem
/// Determines type based on path (trailing slash = directory)
pub fn path_to_tree_item(path: String) -> TreeItem {
    use sha2::{Digest, Sha256};

    let is_dir = path.ends_with('/');
    let trimmed_path = path.trim_end_matches('/').to_string();
    let name = trimmed_path
        .split('/')
        .next_back()
        .unwrap_or(&trimmed_path)
        .to_string();

    // Generate a unique ID from the path hash
    let mut hasher = Sha256::new();
    hasher.update(trimmed_path.as_bytes());
    let id = format!("{:x}", hasher.finalize())[..16].to_string();

    TreeItem {
        id,
        name,
        item_type: if is_dir {
            "tree".to_string()
        } else {
            "blob".to_string()
        },
        path: trimmed_path,
        mode: if is_dir {
            "040000".to_string()
        } else {
            "100644".to_string()
        },
    }
}

/// Git reference response (for creating branches)
#[derive(Debug, Clone, Deserialize)]
pub struct GitRefResponse {
    /// Reference name (e.g., "refs/heads/main")
    #[serde(rename = "ref")]
    pub ref_name: String,
    /// Node ID
    pub node_id: String,
    /// Reference object (commit SHA)
    pub object: GitRefObject,
}

/// Git reference object
#[derive(Debug, Clone, Deserialize)]
pub struct GitRefObject {
    /// Object type (usually "commit")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Object SHA
    pub sha: String,
    /// API URL
    pub url: String,
}

/// Branch information response (from GET /branches/{branch})
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeBranchResponse {
    /// Branch name
    pub name: String,
    /// Commit information
    pub commit: GitCodeBranchCommitWrapper,
}

/// Wrapper for commit in branch response (GitCode API has nested structure)
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeBranchCommitWrapper {
    /// Commit SHA (for GET /branches/{branch})
    #[serde(default)]
    pub id: Option<String>,
    /// Message (for GET /branches/{branch})
    #[serde(default)]
    pub message: Option<String>,
    /// SHA (alternate field name used by POST /branches)
    #[serde(default)]
    pub sha: Option<String>,
    /// Nested commit object (for POST /branches)
    #[serde(default)]
    pub commit: Option<GitCodeBranchCommit>,
}

/// Commit information in branch response
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeBranchCommit {
    /// Commit SHA
    #[serde(default)]
    pub sha: Option<String>,
    /// Commit message
    #[serde(default)]
    pub message: Option<String>,
    /// Author information
    #[serde(default)]
    pub author: Option<GitCodeAuthor>,
    /// Committer information
    #[serde(default)]
    pub committer: Option<GitCodeAuthor>,
    /// Parent commit IDs
    #[serde(default)]
    pub parent_ids: Vec<String>,
    /// Authored date
    #[serde(default)]
    pub authored_date: Option<String>,
}

impl From<GitRefResponse> for Branch {
    fn from(git_ref: GitRefResponse) -> Self {
        // Extract branch name from "refs/heads/branch-name"
        let name = git_ref
            .ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&git_ref.ref_name)
            .to_string();

        Branch {
            name,
            commit: Commit {
                id: git_ref.object.sha,
                message: String::new(), // Not provided in ref response
                author: None,
                timestamp: None,
            },
        }
    }
}

/// File content response from GitCode
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeContentResponse {
    /// Content type (usually "file")
    #[serde(rename = "type")]
    pub content_type: String,
    /// File encoding (usually "base64")
    pub encoding: String,
    /// Size in bytes
    pub size: u64,
    /// File name
    pub name: String,
    /// File path
    pub path: String,
    /// Content (Base64 encoded)
    pub content: String,
    /// File SHA
    pub sha: String,
    /// API URL
    pub url: String,
    /// Download URL
    pub download_url: String,
}

impl From<GitCodeContentResponse> for FileInfo {
    fn from(content: GitCodeContentResponse) -> Self {
        FileInfo {
            name: Some(content.name),
            path: Some(content.path),
            size: Some(content.size),
            content: Some(content.content),
            sha: Some(content.sha),
        }
    }
}

/// Commit creation response
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeCommitResponse {
    /// Content information (optional, not always returned)
    #[serde(default)]
    pub content: Option<GitCodeContentInfo>,
    /// Commit details
    pub commit: GitCodeCommitDetail,
}

/// Content info in commit response
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeContentInfo {
    /// File name
    pub name: String,
    /// File path
    pub path: String,
    /// File SHA
    pub sha: String,
    /// Size in bytes
    pub size: Option<u64>,
    /// HTML URL
    pub html_url: String,
}

/// Detailed commit information
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeCommitDetail {
    /// Commit SHA
    pub sha: String,
    /// Node ID (optional, not always returned by GitCode)
    #[serde(default)]
    pub node_id: Option<String>,
    /// Commit message (optional, not always returned)
    #[serde(default)]
    pub message: Option<String>,
    /// Author information
    pub author: GitCodeAuthor,
    /// Committer information
    pub committer: GitCodeAuthor,
}

/// Author/Committer information
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeAuthor {
    /// Name
    pub name: String,
    /// Email
    pub email: String,
    /// Date
    pub date: String,
}

impl From<GitCodeCommitResponse> for FileCommitResponse {
    fn from(response: GitCodeCommitResponse) -> Self {
        let content_info = response.content.unwrap_or_else(|| GitCodeContentInfo {
            name: String::new(),
            path: String::new(),
            sha: response.commit.sha.clone(),
            size: None,
            html_url: String::new(),
        });

        FileCommitResponse {
            content: FileContent {
                name: content_info.name,
                path: content_info.path,
                sha: content_info.sha,
                size: content_info.size,
                download_url: Some(content_info.html_url),
            },
            commit: Commit {
                id: response.commit.sha,
                message: response.commit.message.unwrap_or_default(),
                author: Some(Author {
                    name: response.commit.author.name,
                    email: response.commit.author.email,
                    date: Some(response.commit.author.date),
                }),
                timestamp: Some(response.commit.committer.date),
            },
        }
    }
}

/// Pull Request creation response
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodePullResponse {
    /// PR number (GitHub-style)
    #[serde(default)]
    pub number: Option<u64>,
    /// Node ID (GitHub-style, optional)
    #[serde(default)]
    pub node_id: Option<String>,
    /// PR ID (GitLab/GitCode-style)
    #[serde(default)]
    pub id: Option<u64>,
    /// Internal ID (GitLab/GitCode-style)
    #[serde(default)]
    pub iid: Option<u64>,
    /// Project ID (GitLab/GitCode-style)
    #[serde(default)]
    pub project_id: Option<u64>,
    /// PR title
    pub title: String,
    /// PR body/description (GitHub-style)
    #[serde(default)]
    pub body: Option<String>,
    /// PR description (GitLab/GitCode-style)
    #[serde(default)]
    pub description: Option<String>,
    /// PR state
    pub state: String,
    /// HTML URL (GitHub-style, optional)
    #[serde(default)]
    pub html_url: Option<String>,
    /// Web URL (GitLab/GitCode-style, optional)
    #[serde(default)]
    pub web_url: Option<String>,
    /// Head reference (source branch, optional)
    #[serde(default)]
    pub head: Option<GitCodePullRef>,
    /// Base reference (target branch, optional)
    #[serde(default)]
    pub base: Option<GitCodePullRef>,
    /// Source branch (GitLab/GitCode-style, optional)
    #[serde(default)]
    pub source_branch: Option<String>,
    /// Target branch (GitLab/GitCode-style, optional)
    #[serde(default)]
    pub target_branch: Option<String>,
    /// User who created the PR (optional)
    #[serde(default)]
    pub user: Option<GitCodeUser>,
    /// Author (GitLab/GitCode-style, optional)
    #[serde(default)]
    pub author: Option<GitCodeUser>,
}

/// PR head/base reference
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodePullRef {
    /// Branch label (e.g., "owner:branch", optional)
    #[serde(default)]
    pub label: Option<String>,
    /// Branch reference (e.g., "refs/heads/branch", GitHub-style)
    #[serde(rename = "ref", default)]
    pub ref_name: Option<String>,
    /// Branch name (GitLab/GitCode-style, optional)
    #[serde(default)]
    pub name: Option<String>,
    /// Commit SHA
    pub sha: String,
    /// Repository info (optional)
    #[serde(default)]
    pub repo: Option<GitCodeRepoInfo>,
}

/// User information
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeUser {
    /// User login/username (optional, GitCode may use different field names)
    #[serde(default)]
    pub login: Option<String>,
    /// User name (alternative to login)
    #[serde(default)]
    pub name: Option<String>,
    /// Username (another alternative)
    #[serde(default)]
    pub username: Option<String>,
    /// User ID (optional)
    #[serde(default)]
    pub id: Option<u64>,
    /// Avatar URL (optional)
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// HTML URL (optional)
    #[serde(default)]
    pub html_url: Option<String>,
}

/// Repository information
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeRepoInfo {
    /// Repository ID
    pub id: u64,
    /// Repository name
    pub name: String,
    /// Full name (owner/repo)
    pub full_name: String,
    /// Private flag (optional)
    #[serde(default)]
    pub private: Option<bool>,
    /// Owner information (optional)
    #[serde(default)]
    pub owner: Option<GitCodeUser>,
    /// HTML URL (optional)
    #[serde(default)]
    pub html_url: Option<String>,
    /// Description (optional)
    #[serde(default)]
    pub description: Option<String>,
    /// Default branch (optional)
    #[serde(default)]
    pub default_branch: Option<String>,
    /// Clone URL (HTTPS, optional)
    #[serde(default)]
    pub clone_url: Option<String>,
    /// SSH URL (optional)
    #[serde(default)]
    pub ssh_url: Option<String>,
}

/// Repository response wrapper
#[derive(Debug, Clone, Deserialize)]
pub struct GitCodeRepoResponse {
    /// Repository ID
    pub id: u64,
    /// Repository name
    pub name: String,
    /// Full name (owner/repo)
    pub full_name: String,
    /// Private flag
    pub private: bool,
    /// Owner information
    pub owner: GitCodeUser,
    /// HTML URL
    pub html_url: String,
    /// Description
    pub description: Option<String>,
    /// Default branch
    pub default_branch: String,
    /// Clone URL (HTTPS)
    pub clone_url: String,
    /// SSH URL
    pub ssh_url: String,
}

impl From<GitCodePullResponse> for PullRequest {
    fn from(pr: GitCodePullResponse) -> Self {
        // Prefer GitHub-style fields, fall back to GitLab-style
        let number = pr.number.or(pr.iid).unwrap_or(0);
        let html_url = pr.html_url.or(pr.web_url);
        let body = pr.body.or(pr.description);

        // Extract branch names - prefer head/base, fallback to source/target branch
        let head_info = if let Some(head) = &pr.head {
            let branch_name = head
                .ref_name
                .as_ref()
                .or(head.name.as_ref())
                .cloned()
                .unwrap_or_else(|| format!("sha:{}", head.sha));
            Some(PullRequestRef {
                ref_branch: branch_name,
                repo: head.repo.as_ref().map(|r| RepositoryRef {
                    full_name: r.full_name.clone(),
                }),
            })
        } else {
            pr.source_branch
                .as_ref()
                .map(|source_branch| PullRequestRef {
                    ref_branch: source_branch.clone(),
                    repo: None,
                })
        };

        let base_info = if let Some(base) = &pr.base {
            let branch_name = base
                .ref_name
                .as_ref()
                .or(base.name.as_ref())
                .cloned()
                .unwrap_or_else(|| format!("sha:{}", base.sha));
            Some(PullRequestRef {
                ref_branch: branch_name,
                repo: base.repo.as_ref().map(|r| RepositoryRef {
                    full_name: r.full_name.clone(),
                }),
            })
        } else {
            pr.target_branch
                .as_ref()
                .map(|target_branch| PullRequestRef {
                    ref_branch: target_branch.clone(),
                    repo: None,
                })
        };

        PullRequest {
            number,
            title: pr.title,
            html_url,
            state: pr.state,
            head: head_info,
            base: base_info,
            body,
        }
    }
}

impl From<GitCodeRepoResponse> for RepositoryInfo {
    fn from(repo: GitCodeRepoResponse) -> Self {
        RepositoryInfo {
            id: repo.id,
            full_name: repo.full_name,
            description: repo.description,
            default_branch: repo.default_branch,
            private: Some(repo.private),
            html_url: Some(repo.html_url),
            clone_url: Some(repo.clone_url),
            ssh_url: Some(repo.ssh_url),
        }
    }
}

/// Request body for creating a branch (GitCode/Gitee API)
#[derive(Debug, Clone, Serialize)]
pub struct CreateBranchRequest {
    /// Branch name (without refs/heads/ prefix)
    pub branch_name: String,
    /// Source branch name or commit SHA
    pub refs: String,
}

/// Request body for creating/updating a file
#[derive(Debug, Clone, Serialize)]
pub struct CreateFileRequest {
    /// Commit message
    pub message: String,
    /// Base64-encoded content
    pub content: String,
    /// Target branch
    pub branch: String,
    /// File SHA (required for updates with PUT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// Author name (GitCode format: author[name])
    #[serde(rename = "author[name]", skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    /// Author email (GitCode format: author[email])
    #[serde(rename = "author[email]", skip_serializing_if = "Option::is_none")]
    pub author_email: Option<String>,
}

/// Committer information for requests
#[derive(Debug, Clone, Serialize)]
pub struct GitCodeCommitter {
    /// Committer name
    pub name: String,
    /// Committer email
    pub email: String,
}

/// Request body for creating a pull request
#[derive(Debug, Clone, Serialize)]
pub struct CreatePullRequest {
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

/// Request body for assigning reviewers
#[derive(Debug, Clone, Serialize)]
pub struct RequestReviewers {
    /// List of reviewer usernames
    pub reviewers: Vec<String>,
}

#[cfg(test)]
mod tests {}
