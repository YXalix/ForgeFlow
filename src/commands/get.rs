//! get command implementation
//!
//! Fetch remote files or directories

use crate::api::ApiClient;
use crate::api::types::TreeItem;
use crate::cli::GetArgs;
use crate::commands::Command;
use crate::config::Config;
use crate::error::{Result, VktError};
use colored::Colorize;
use std::path::Path;

/// get command
pub struct GetCommand {
    args: GetArgs,
}

/// Download result
#[derive(Debug)]
struct DownloadResult {
    path: String,
    success: bool,
    size: usize,
    error: Option<String>,
}

impl GetCommand {
    /// Create a new get command
    pub fn new(args: GetArgs) -> Self {
        Self { args }
    }

    /// Get file name
    fn get_file_name<'a>(&self, remote_path: &'a str) -> &'a str {
        Path::new(remote_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(remote_path)
    }

    /// Ensure output directory exists
    fn ensure_output_dir(&self, output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(VktError::Io)?;
        }
        Ok(())
    }

    /// Check if file exists and handle accordingly
    fn check_existing_file(&self, output_path: &Path) -> Result<bool> {
        if output_path.exists() && !self.args.force {
            return Err(VktError::Validation(format!(
                "File '{}' already exists, use -f/--force to overwrite",
                output_path.display()
            )));
        }
        Ok(true)
    }

    /// Download a single file
    async fn download_file(
        &self,
        client: &ApiClient,
        remote_path: &str,
        local_path: &Path,
        branch: &str,
    ) -> Result<usize> {
        let content = client.get_file_raw(remote_path, Some(branch)).await?;
        let size = content.len();

        // Ensure parent directory exists
        if let Some(parent) = local_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(local_path, &content)?;
        Ok(size)
    }

    /// Download file task (for concurrent downloads)
    async fn download_file_task(
        config: Config,
        remote_path: String,
        local_path: std::path::PathBuf,
        branch: String,
        force: bool,
    ) -> DownloadResult {
        // Check if file already exists
        if local_path.exists() && !force {
            return DownloadResult {
                path: remote_path,
                success: false,
                size: 0,
                error: Some(format!(
                    "File '{}' already exists, use -f to overwrite",
                    local_path.display()
                )),
            };
        }

        // Create client inside the task
        let client = match ApiClient::new(&config) {
            Ok(c) => c,
            Err(e) => {
                return DownloadResult {
                    path: remote_path,
                    success: false,
                    size: 0,
                    error: Some(format!("Failed to create API client: {}", e)),
                };
            }
        };

        match client.get_file_raw(&remote_path, Some(&branch)).await {
            Ok(content) => {
                let size = content.len();

                // Ensure parent directory exists
                if let Some(parent) = local_path.parent()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    return DownloadResult {
                        path: remote_path,
                        success: false,
                        size: 0,
                        error: Some(format!("Failed to create directory: {}", e)),
                    };
                }

                match std::fs::write(&local_path, &content) {
                    Ok(_) => DownloadResult {
                        path: remote_path,
                        success: true,
                        size,
                        error: None,
                    },
                    Err(e) => DownloadResult {
                        path: remote_path,
                        success: false,
                        size: 0,
                        error: Some(format!("Failed to write file: {}", e)),
                    },
                }
            }
            Err(e) => DownloadResult {
                path: remote_path,
                success: false,
                size: 0,
                error: Some(e.to_string()),
            },
        }
    }

    /// Get all files in directory
    async fn download_directory(
        &self,
        client: &ApiClient,
        remote_dir: &str,
        output_dir: &Path,
        branch: &str,
        config: &Config,
    ) -> Result<Vec<DownloadResult>> {
        // Get all items in directory
        let items = client
            .list_repository_tree(Some(remote_dir), true, Some(branch))
            .await?;

        // Filter to files only
        let files: Vec<&TreeItem> = items.iter().filter(|item| item.is_file()).collect();

        if files.is_empty() {
            println!("{} Directory is empty: {}", "INFO:".blue(), remote_dir);
            return Ok(Vec::new());
        }

        let total_files = files.len();
        println!(
            "{} Found {} files, starting concurrent download...",
            "→".blue(),
            total_files.to_string().cyan()
        );

        let branch = branch.to_string();
        let force = self.args.force;
        let base_remote_dir = remote_dir.to_string();
        let config = config.clone();

        // Create download tasks
        let mut tasks = Vec::new();

        for file in files {
            let remote_path = file.path.clone();
            let relative_path = if remote_path.starts_with(&base_remote_dir) {
                remote_path[base_remote_dir.len()..].trim_start_matches('/')
            } else {
                &remote_path
            };

            let local_path = output_dir.join(relative_path);
            let config = config.clone();
            let branch = branch.clone();
            let remote_path_clone = remote_path.clone();

            let task = tokio::spawn(async move {
                Self::download_file_task(config, remote_path_clone, local_path, branch, force).await
            });

            tasks.push((remote_path, task));
        }

        // Collect results
        let mut results = Vec::new();
        let mut completed = 0;

        for (remote_path, task) in tasks {
            match task.await {
                Ok(result) => {
                    completed += 1;
                    if result.success {
                        let size_str = if result.size < 1024 {
                            format!("{}B", result.size)
                        } else {
                            format!("{:.1}KB", result.size as f64 / 1024.0)
                        };
                        println!(
                            "  {} {} ({})",
                            "✓".green(),
                            remote_path.green(),
                            size_str.yellow()
                        );
                    } else {
                        println!(
                            "  {} {} - {}",
                            "✗".red(),
                            remote_path.red(),
                            result
                                .error
                                .as_ref()
                                .unwrap_or(&"Unknown error".to_string())
                        );
                    }
                    results.push(result);
                }
                Err(e) => {
                    completed += 1;
                    println!("  {} {} - Task error: {}", "✗".red(), remote_path.red(), e);
                    results.push(DownloadResult {
                        path: remote_path,
                        success: false,
                        size: 0,
                        error: Some(format!("Task error: {}", e)),
                    });
                }
            }

            // Show progress
            print!("\r  Progress: {}/{} files", completed, total_files);
        }

        println!(); // New line

        Ok(results)
    }

    /// Format byte size
    fn format_bytes(bytes: usize) -> String {
        if bytes < 1024 {
            format!("{}B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1}KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

#[async_trait::async_trait]
impl Command for GetCommand {
    async fn execute(&self) -> Result<()> {
        let config = Config::load()?;
        let client = ApiClient::new(&config)?;

        let remote_path = &self.args.path;
        let output_dir = self.args.output.as_deref().unwrap_or(".");
        let branch = &config.repo.default_branch;

        // First try to get file info to determine if it's a file or directory
        let is_directory = match client.get_file_info(remote_path, Some(branch)).await {
            Ok(_) => false, // It's a file
            Err(_) => {
                // Might not be a file, try to get directory listing
                match client
                    .list_repository_tree(Some(remote_path), false, Some(branch))
                    .await
                {
                    Ok(items) => !items.is_empty(), // If has content, it's a directory
                    Err(_) => {
                        // Neither file nor directory, probably 404
                        return Err(VktError::Api(format!(
                            "Path '{}' does not exist or cannot be accessed",
                            remote_path
                        )));
                    }
                }
            }
        };

        if is_directory {
            // Handle directory download
            let dir_name = self.get_file_name(remote_path);
            let output_path = Path::new(output_dir).join(dir_name);

            println!("{} Fetching directory: {}", "→".blue(), remote_path.cyan());
            println!(
                "{} Saving to: {}",
                "→".blue(),
                output_path.display().to_string().cyan()
            );

            let results = self
                .download_directory(&client, remote_path, &output_path, branch, &config)
                .await?;

            // Statistics
            let success_count = results.iter().filter(|r| r.success).count();
            let fail_count = results.len() - success_count;
            let total_bytes: usize = results.iter().map(|r| r.size).sum();

            println!();
            if fail_count == 0 {
                println!(
                    "{} Download complete: {} files, total {}",
                    "✓".green(),
                    success_count.to_string().green(),
                    Self::format_bytes(total_bytes).yellow()
                );
            } else {
                println!(
                    "{} Download complete: {} succeeded, {} failed, total {}",
                    "⚠".yellow(),
                    success_count.to_string().green(),
                    fail_count.to_string().red(),
                    Self::format_bytes(total_bytes).yellow()
                );

                // Show failed files
                println!("\nFailed files:");
                for result in &results {
                    if !result.success {
                        println!(
                            "  - {}: {}",
                            result.path.red(),
                            result
                                .error
                                .as_ref()
                                .unwrap_or(&"Unknown error".to_string())
                        );
                    }
                }

                if success_count == 0 {
                    return Err(VktError::Api("All files failed to download".to_string()));
                }
            }
        } else {
            // Handle single file download
            let file_name = self.get_file_name(remote_path);
            let output_path = Path::new(output_dir).join(file_name);

            // Ensure output directory exists
            self.ensure_output_dir(&output_path)?;

            // Check if file exists
            self.check_existing_file(&output_path)?;

            // Download file
            println!("{} Fetching: {}", "→".blue(), remote_path.cyan());

            let size = self
                .download_file(&client, remote_path, &output_path, branch)
                .await?;

            println!(
                "{} Saved: {} ({})",
                "✓".green(),
                output_path.display().to_string().green(),
                Self::format_bytes(size).yellow()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_name() {
        let cmd = GetCommand::new(GetArgs {
            path: "scripts/config.sh".to_string(),
            output: None,
            force: false,
        });
        assert_eq!(cmd.get_file_name("scripts/config.sh"), "config.sh");

        let cmd2 = GetCommand::new(GetArgs {
            path: "README.md".to_string(),
            output: None,
            force: false,
        });
        assert_eq!(cmd2.get_file_name("README.md"), "README.md");
    }

    #[test]
    fn test_get_file_name_with_special_chars() {
        let cmd = GetCommand::new(GetArgs {
            path: "path/to/file-name_v1.0.txt".to_string(),
            output: None,
            force: false,
        });
        assert_eq!(
            cmd.get_file_name("path/to/file-name_v1.0.txt"),
            "file-name_v1.0.txt"
        );
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(GetCommand::format_bytes(500), "500B");
        assert_eq!(GetCommand::format_bytes(1024), "1.0KB");
        assert_eq!(GetCommand::format_bytes(1536), "1.5KB");
        assert_eq!(GetCommand::format_bytes(1024 * 1024), "1.0MB");
    }
}
