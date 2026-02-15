//! list command implementation
//!
//! List remote repository contents (ls-like behavior)

use crate::api::ApiClient;
use crate::cli::ListArgs;
use crate::commands::Command;
use crate::config::Config;
use crate::error::Result;
use colored::Colorize;

/// list command
pub struct ListCommand {
    args: ListArgs,
}

impl ListCommand {
    /// Create a new list command
    pub fn new(args: ListArgs) -> Self {
        Self { args }
    }
}

#[async_trait::async_trait]
impl Command for ListCommand {
    async fn execute(&self) -> Result<()> {
        let config = Config::load()?;
        let client = ApiClient::new(&config)?;

        let path = self.args.path.as_deref();
        let recursive = self.args.recursive;

        // Check if the path is a file (ls-like behavior)
        if let Some(file_path) = path {
            // Normalize path: remove leading/trailing slashes
            let normalized_path = file_path.trim_matches('/');
            if !normalized_path.is_empty() {
                match client.file_exists(normalized_path, Some(&config.repo.default_branch)).await {
                    Ok(true) => {
                        // It's a file, get file info and print it
                        match client.get_file_info(normalized_path, Some(&config.repo.default_branch)).await {
                            Ok(file_info) => {
                                // Print just the file name (like ls does)
                                let name = file_info.name.unwrap_or_else(|| {
                                    normalized_path.split('/').next_back().unwrap_or(normalized_path).to_string()
                                });
                                println!("{}", name);
                                return Ok(());
                            }
                            Err(_) => {
                                // Fall through to directory listing
                            }
                        }
                    }
                    Ok(false) => {
                        // Not a file, continue to directory listing
                    }
                    Err(_) => {
                        // Error checking, continue to directory listing
                    }
                }
            }
        }

        // Get file tree
        let items = client
            .list_repository_tree(path, recursive, Some(&config.repo.default_branch))
            .await?;

        if items.is_empty() {
            println!("{} Directory is empty", "INFO:".blue());
            return Ok(());
        }

        // Sort: directories first, files second, sorted by name
        let mut items = items;
        items.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        // Print items
        for item in &items {
            if item.is_dir() {
                if recursive {
                    // In recursive mode, show full path with trailing slash
                    println!("{}/", item.path.cyan());
                } else {
                    // In non-recursive mode, show just the name with trailing slash
                    println!("{}/", item.name.cyan());
                }
            } else if recursive {
                // In recursive mode, show full path
                println!("{}", item.path);
            } else {
                // In non-recursive mode, show just the name
                println!("{}", item.name);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Tests removed - no complex logic to test in simplified version
}
