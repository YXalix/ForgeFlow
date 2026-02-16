//! config command implementation
//!
//! Manage VKT configuration (get/set/list)

use crate::cli::ConfigArgs;
use crate::commands::Command;
use crate::config::Config;
use crate::error::{Result, VktError};
use colored::Colorize;

pub struct ConfigCommand {
    args: ConfigArgs,
}

impl ConfigCommand {
    pub fn new(args: ConfigArgs) -> Self {
        Self { args }
    }

    /// Display single config value
    fn show_value(&self, config: &Config, key: &str) -> Result<()> {
        let value = config.get_value(key)?;
        println!("{}", value);
        Ok(())
    }

    /// Display all config values
    fn list_all(&self, config: &Config) {
        println!("{}", "[user]".cyan());
        println!("  name = {}", config.user.name.green());
        println!("  email = {}", config.user.email.green());
        println!(
            "  auto_signoff = {}",
            config.user.auto_signoff.to_string().yellow()
        );

        println!("{}", "[remote]".cyan());
        println!("  provider = {}", config.remote.provider.green());
        println!("  api_url = {}", config.remote.api_url.green());
        println!("  token = {}", "********".green());

        println!("{}", "[repo]".cyan());
        println!("  project_id = {}", config.repo.project_id.green());
        println!("  default_branch = {}", config.repo.default_branch.green());

        println!("{}", "[template]".cyan());
        println!("  pr_prefix = {}", config.template.pr_prefix.green());
    }
}

#[async_trait::async_trait]
impl Command for ConfigCommand {
    async fn execute(&self) -> Result<()> {
        let config_path = Config::default_path()?;

        // Handle setup mode
        if self.args.setup {
            let config = Config::interactive_setup()?;
            Config::ensure_config_dir()?;
            config.save_to_file(&config_path)?;
            println!(
                "{} Configuration saved to {}",
                "✓".green(),
                config_path.to_string_lossy().cyan()
            );
            return Ok(());
        }

        // Load or check if config exists
        let mut config = if config_path.exists() {
            Config::load()
        } else {
            println!(
                "{} No configuration found at {}",
                "INFO:".blue(),
                config_path.to_string_lossy()
            );
            println!();
            println!("Run {} to create one.", "vkt config --setup".cyan());
            return Ok(());
        }?;

        // Handle list mode (no args or --list flag)
        if self.args.list || (self.args.key.is_none() && self.args.value.is_none()) {
            self.list_all(&config);
            return Ok(());
        }

        let key = self
            .args
            .key
            .as_ref()
            .ok_or_else(|| VktError::Validation("Key is required".to_string()))?;

        // Handle get mode (key only, no value)
        if self.args.value.is_none() {
            return self.show_value(&config, key);
        }

        // Handle set mode (key + value)
        let value = self
            .args
            .value
            .as_ref()
            .ok_or_else(|| VktError::Validation("Value is required".to_string()))?;

        config.set_value(key, value)?;
        config.save_to_file(&config_path)?;
        println!("{} {} = {}", "✓".green(), key.yellow(), value.green());

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

    #[test]
    fn test_config_command_list_all() {
        let args = ConfigArgs {
            key: None,
            value: None,
            list: false,
            setup: false,
        };
        let cmd = ConfigCommand::new(args);
        let config = create_test_config();

        // Should not panic
        cmd.list_all(&config);
    }

    #[test]
    fn test_config_command_show_value() {
        let args = ConfigArgs {
            key: Some("user.name".to_string()),
            value: None,
            list: false,
            setup: false,
        };
        let cmd = ConfigCommand::new(args);
        let config = create_test_config();

        assert!(cmd.show_value(&config, "user.name").is_ok());
    }

    #[test]
    fn test_config_command_show_value_unknown_key() {
        let args = ConfigArgs {
            key: Some("unknown.key".to_string()),
            value: None,
            list: false,
            setup: false,
        };
        let cmd = ConfigCommand::new(args);
        let config = create_test_config();

        assert!(cmd.show_value(&config, "unknown.key").is_err());
    }
}
