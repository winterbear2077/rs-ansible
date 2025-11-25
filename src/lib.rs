pub mod error;
pub mod types;
pub mod ssh;
pub mod manager;
pub mod config;
pub mod executor;
pub mod utils;

#[cfg(test)]
mod tests;

pub use error::AnsibleError;
pub use types::{
    HostConfig, SystemInfo, CommandResult, FileTransferResult, NetworkInterface, FileCopyOptions,
    UserOptions, UserResult, UserInfo, UserState,
    TemplateOptions, TemplateResult,
};
pub use ssh::SshClient;
pub use manager::{AnsibleManager, BatchResult, HostConfigBuilder, BatchOperationStats};
pub use config::InventoryConfig;
pub use executor::{TaskExecutor, Task, Playbook, TaskType, TaskResult, PlaybookResult};

// 便捷的重新导出
pub type Result<T> = std::result::Result<T, AnsibleError>;