use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            port: 22,
            username: String::new(),
            password: None,
            private_key_path: None,
            passphrase: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub kernel_version: String,
    pub architecture: String,
    pub uptime: String,
    pub memory_total: String,
    pub memory_free: String,
    pub disk_usage: HashMap<String, String>,
    pub cpu_info: String,
    pub network_interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: String,
    pub mac_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferResult {
    pub success: bool,
    pub bytes_transferred: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCopyOptions {
    pub owner: Option<String>,
    pub group: Option<String>,
    pub mode: Option<String>, // 文件权限，例如 "644", "755"
    pub backup: bool,         // 是否在覆盖前备份
    pub create_dirs: bool,    // 是否创建目标目录
    #[serde(default = "default_verify_hash")]
    pub verify_hash: bool,    // 是否验证文件hash（幂等性检查）
    pub hash_algorithm: Option<String>, // hash算法: sha256, md5 等
}

fn default_verify_hash() -> bool {
    true // 默认启用hash验证
}

impl Default for FileCopyOptions {
    fn default() -> Self {
        Self {
            owner: None,
            group: None,
            mode: Some("644".to_string()), // 默认权限
            backup: false,
            create_dirs: true,
            verify_hash: true,  // 默认启用hash验证
            hash_algorithm: Some("sha256".to_string()), // 默认使用sha256
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashInfo {
    pub algorithm: String,
    pub hash: String,
    pub size: u64,
}