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
    /// 预先计算的本地文件 Hash (SHA256)。如果提供，将跳过本地计算步骤。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precomputed_hash: Option<String>,
}

impl Default for FileCopyOptions {
    fn default() -> Self {
        Self {
            owner: None,
            group: None,
            mode: Some("644".to_string()), // 默认权限
            backup: false,
            create_dirs: true,
            precomputed_hash: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashInfo {
    pub algorithm: String,
    pub hash: String,
    pub size: u64,
}

/// 用户管理选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOptions {
    pub name: String,                    // 用户名
    pub state: UserState,                // 用户状态: present 或 absent
    pub uid: Option<u32>,                // 用户ID
    pub group: Option<String>,           // 主组
    pub groups: Option<Vec<String>>,     // 附加组
    pub home: Option<String>,            // 家目录
    pub shell: Option<String>,           // 登录shell
    pub password: Option<String>,        // 密码（已加密）
    pub comment: Option<String>,         // 用户描述
    pub create_home: bool,               // 是否创建家目录
    pub system: bool,                    // 是否为系统用户
    pub expires: Option<String>,         // 账户过期时间
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserState {
    Present,  // 确保用户存在
    Absent,   // 确保用户不存在
}

impl Default for UserOptions {
    fn default() -> Self {
        Self {
            name: String::new(),
            state: UserState::Present,
            uid: None,
            group: None,
            groups: None,
            home: None,
            shell: Some("/bin/bash".to_string()),
            password: None,
            comment: None,
            create_home: true,
            system: false,
            expires: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResult {
    pub success: bool,
    pub changed: bool,    // 是否做了改变
    pub message: String,
    pub user_info: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    pub comment: String,
}

/// 模板渲染选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateOptions {
    pub src: String,                     // 模板文件路径（本地）
    pub dest: String,                    // 目标文件路径（远程）
    pub variables: HashMap<String, serde_json::Value>,  // ✅ 支持任意 JSON 值（字符串、数字、数组、对象等）
    pub owner: Option<String>,           // 文件所有者
    pub group: Option<String>,           // 文件组
    pub mode: Option<String>,            // 文件权限
    pub backup: bool,                    // 是否备份现有文件
    pub validate: Option<String>,        // 验证命令（在替换前验证文件）
}

impl Default for TemplateOptions {
    fn default() -> Self {
        Self {
            src: String::new(),
            dest: String::new(),
            variables: HashMap::new(),
            owner: None,
            group: None,
            mode: Some("644".to_string()),
            backup: false,
            validate: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateResult {
    pub success: bool,
    pub changed: bool,     // 文件是否被改变
    pub message: String,
    pub diff: Option<String>,  // 文件差异（如果可用）
}