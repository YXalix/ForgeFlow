//! Command implementation module
//!
//! Contains implementations for list, get, and submit commands

pub mod config;
pub mod get;
pub mod list;
pub mod submit;

use anyhow::Result;

/// Command trait
#[async_trait::async_trait]
pub trait Command {
    /// Execute the command
    async fn execute(&self) -> Result<()>;
}
