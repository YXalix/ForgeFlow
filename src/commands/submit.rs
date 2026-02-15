//! submit command implementation
//!
//! Submit files to remote repository and create PR

use crate::api::ApiClient;
use crate::cli::SubmitArgs;
use crate::commands::Command;
use crate::config::Config;
use crate::error::{Result, VktError};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs;

/// submit command
pub struct SubmitCommand {
    args: SubmitArgs,
}

impl SubmitCommand {
    /// Create a new submit command
    pub fn new(args: SubmitArgs) -> Self {
        Self { args }
    }

    /// Generate branch name
    fn generate_branch_name(&self, msg: &str) -> String {
        // If user specified branch name, use it directly
        if let Some(ref branch) = self.args.branch {
            return branch.clone();
        }

        // Auto-generate branch name: feat/submit-{timestamp}-{msg-prefix}
        let timestamp = chrono::Local::now().timestamp();
        let msg_prefix = msg
            .split_whitespace()
            .next()
            .unwrap_or("submit")
            .replace(':', "")
            .replace('/', "-")
            .to_lowercase();

        format!("feat/vkt-submit-{}-{}", timestamp, msg_prefix)
    }

    /// Generate file hash
    fn generate_file_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Generate commit message
    fn generate_commit_message(
        &self,
        msg: &str,
        local_path: &str,
        file_hash: &str,
        config: &Config,
    ) -> String {
        let now = chrono::Local::now().to_rfc3339();

        if config.user.auto_signoff {
            format!(
                "{}\n\nOriginal-File: {}\nOriginal-File-Hash: {}\nDate: {}\nSigned-off-by: {} <{}>\n",
                msg, local_path, file_hash, now, config.user.name, config.user.email
            )
        } else {
            format!(
                "{}\n\nOriginal-File: {}\nOriginal-File-Hash: {}\nDate: {}\n",
                msg, local_path, file_hash, now
            )
        }
    }

    /// Generate PR description
    fn generate_pr_body(
        &self,
        msg: &str,
        local_path: &str,
        file_hash: &str,
        config: &Config,
    ) -> String {
        let now = chrono::Local::now().to_rfc3339();

        format!(
            "## Change Description\n{}\n\n## Trace Information\n- Original File: {}\n- File Hash: {}\n- Submission Time: {}\n- Submitter: {} <{}>",
            msg, local_path, file_hash, now, config.user.name, config.user.email
        )
    }
}

#[async_trait::async_trait]
impl Command for SubmitCommand {
    async fn execute(&self) -> Result<()> {
        // 1. Load configuration
        let config = Config::load()?;
        let api = ApiClient::new(&config)?;

        // 2. Check local file exists
        let local_path = Path::new(&self.args.local_path);
        if !local_path.exists() {
            return Err(VktError::Validation(format!(
                "File does not exist: {}",
                self.args.local_path
            )));
        }

        // Currently only single file submission is supported
        if local_path.is_dir() {
            return Err(VktError::Validation(
                "Directory submission not yet supported, please specify a single file".to_string(),
            ));
        }

        // 3. Generate target path
        let file_name = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| VktError::Validation("Invalid filename".to_string()))?;
        let target_path = format!("{}/{}", self.args.target.trim_end_matches('/'), file_name);

        // 4. Diff Check - check if remote exists
        println!("🔍 Checking if remote file exists: {}", target_path);
        let remote_exists = match api
            .get_file_info(&target_path, Some(&config.repo.default_branch))
            .await
        {
            Ok(_) => true,
            Err(e) if e.is_not_found() => false,
            Err(e) => return Err(e),
        };

        if remote_exists && !self.args.force {
            return Err(VktError::Validation(format!(
                "Remote file already exists: {}. Use --force to overwrite",
                target_path
            )));
        }

        if remote_exists && self.args.force {
            println!("⚠️  Remote file exists, force overwrite mode");
        }

        // 5. Check if repository is empty (no default branch)
        println!("🔍 Checking if default branch exists...");
        let repo_has_commits = match api
            .list_repository_tree(None, false, Some(&config.repo.default_branch))
            .await
        {
            Ok(tree) => !tree.is_empty(), // Empty tree means no commits
            Err(e) if e.is_not_found() => false,
            Err(e) => return Err(e),
        };

        // If repository has no commits, it can't be used with the API workflow
        if !repo_has_commits {
            println!("❌ Repository is empty and must be initialized first");
            println!();
            println!("GitCode requires repositories to be initialized before using the API.");
            println!("Please initialize your repository by:");
            println!("  1. Visit: https://gitcode.com/{}", config.repo.project_id);
            return Err(VktError::Validation(
                "Repository not initialized. Please create a README file via web UI first."
                    .to_string(),
            ));
        }

        println!("✅ Repository has been initialized");
        let target_branch = self.generate_branch_name(&self.args.msg);

        // 6. Dry run mode
        if self.args.dry_run {
            println!("📋 Dry run mode - Planned operations:");
            println!("   - Local file: {}", self.args.local_path);
            println!("   - Target path: {}", target_path);
            println!("   - Commit message: {}", self.args.msg);
            println!(
                "   - Based on branch: {} -> new branch: {}",
                config.repo.default_branch, target_branch
            );
            return Ok(());
        }

        // 7. Create branch
        println!("🌿 Creating branch: {}", target_branch);
        api.create_branch(&target_branch, &config.repo.default_branch)
            .await?;
        println!("✅ Branch created successfully");

        // 8. Read and encode file content
        let content = fs::read(&self.args.local_path).await?;
        let content_hash = Self::generate_file_hash(&content);
        let base64_content = base64::engine::general_purpose::STANDARD.encode(&content);

        println!("📄 File size: {} bytes", content.len());
        println!("🔐 File hash: {}", content_hash);

        // 9. Generate commit message with trace info
        let commit_message = self.generate_commit_message(
            &self.args.msg,
            &self.args.local_path,
            &content_hash,
            &config,
        );

        // 10. Upload file
        println!("⬆️  Uploading file to remote...");
        api.create_or_update_file(
            &target_path,
            &base64_content,
            &target_branch,
            &commit_message,
            &config.user.name,
            &config.user.email,
        )
        .await?;
        println!("✅ File uploaded successfully: {}", target_path);

        // 11. Create PR
        let pr_title = format!("{} {}", config.template.pr_prefix, self.args.msg);
        let pr_body = self.generate_pr_body(
            &self.args.msg,
            &self.args.local_path,
            &content_hash,
            &config,
        );

        println!("📨 Creating Pull Request...");
        let pr = api
            .create_pull_request(
                &pr_title,
                &target_branch,
                &config.repo.default_branch,
                Some(&pr_body),
            )
            .await?;
        println!("✅ PR #{} created successfully: {}", pr.number, pr.title);

        // 13. Output results
        println!();
        println!("🎉 Submission complete!");
        println!("   File: {}", target_path);
        println!("   Branch: {}", target_branch);
        println!("   PR #{}: {}", pr.number, pr.title);
        println!("   Link: {}", pr.html_url.as_deref().unwrap_or("N/A"));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            user: crate::config::UserConfig {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                auto_signoff: true,
            },
            remote: crate::config::RemoteConfig {
                provider: "Gitcode".to_string(),
                api_url: "https://api.example.com".to_string(),
                token: "test-token".to_string(),
            },
            repo: crate::config::RepoConfig {
                project_id: "owner/repo".to_string(),
                default_branch: "main".to_string(),
            },
            template: crate::config::TemplateConfig {
                pr_prefix: "[TEST]".to_string(),
            },
        }
    }

    fn create_test_args() -> SubmitArgs {
        SubmitArgs {
            local_path: "./test.sh".to_string(),
            target: "scripts/".to_string(),
            msg: "feat: add test script".to_string(),
            force: false,
            dry_run: false,
            branch: None,
        }
    }

    #[test]
    fn test_generate_branch_name_auto() {
        let args = create_test_args();
        let cmd = SubmitCommand::new(args);
        let branch = cmd.generate_branch_name("feat: add new feature");

        assert!(branch.starts_with("feat/vkt-submit-"));
        assert!(branch.contains("feat"));
    }

    #[test]
    fn test_generate_branch_name_custom() {
        let mut args = create_test_args();
        args.branch = Some("custom/branch".to_string());
        let cmd = SubmitCommand::new(args);
        let branch = cmd.generate_branch_name("feat: test");

        assert_eq!(branch, "custom/branch");
    }

    #[test]
    fn test_generate_file_hash() {
        let content = b"test content";
        let hash = SubmitCommand::generate_file_hash(content);

        assert_eq!(hash.len(), 64); // SHA256 hex length
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_commit_message_with_signoff() {
        let args = create_test_args();
        let cmd = SubmitCommand::new(args);
        let config = create_test_config();
        let msg = cmd.generate_commit_message("feat: test", "./test.sh", "abc123", &config);

        assert!(msg.contains("feat: test"));
        assert!(msg.contains("Original-File: ./test.sh"));
        assert!(msg.contains("Original-File-Hash: abc123"));
        assert!(msg.contains("Signed-off-by: Test User <test@example.com>"));
    }

    #[test]
    fn test_generate_commit_message_without_signoff() {
        let args = create_test_args();
        let cmd = SubmitCommand::new(args);
        let mut config = create_test_config();
        config.user.auto_signoff = false;
        let msg = cmd.generate_commit_message("feat: test", "./test.sh", "abc123", &config);

        assert!(msg.contains("feat: test"));
        assert!(msg.contains("Original-File: ./test.sh"));
        assert!(!msg.contains("Signed-off-by"));
    }

    #[test]
    fn test_generate_pr_body() {
        let args = create_test_args();
        let cmd = SubmitCommand::new(args);
        let config = create_test_config();
        let body = cmd.generate_pr_body("feat: test", "./test.sh", "abc123", &config);

        assert!(body.contains("## Change Description"));
        assert!(body.contains("feat: test"));
        assert!(body.contains("## Trace Information"));
        assert!(body.contains("- Original File: ./test.sh"));
        assert!(body.contains("- File Hash: abc123"));
        assert!(body.contains("- Submitter: Test User <test@example.com>"));
    }

    #[test]
    fn test_branch_name_sanitization() {
        let args = create_test_args();
        let cmd = SubmitCommand::new(args);

        // Test commit message with special characters - colons should be removed
        let branch = cmd.generate_branch_name("feat/fix: something/bug");
        assert!(branch.contains("feat-fix")); // Colons and slashes in message are cleaned
        assert!(!branch.contains(':')); // No colon
        // Note: Branch format is "feat/vkt-submit-{timestamp}-{msg_prefix}", slash in prefix is expected
        assert!(branch.starts_with("feat/vkt-submit-")); // Branch prefix remains unchanged
    }
}
