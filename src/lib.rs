//! VKT (Virt-Kernel-Tools)
//!
//! A CLI tool designed for virtualization and kernel development teams.
//! Wraps Git/Forge APIs to streamline script distribution, configuration retrieval,
//! and code submission workflows.

pub mod api;
pub mod cli;
pub mod commands;
pub mod config;
pub mod error;

pub use error::{Result, VktError};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const NAME: &str = env!("CARGO_PKG_NAME");
