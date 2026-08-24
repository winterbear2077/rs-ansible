//! Go one-shot agent（`adms-agent`）的最小 Rust 客户端。
//!
//! Rust installer 在更新时把 Go 侧编译的 agent 二进制传输到目标主机，然后
//! 通过 JSON stdin / NDJSON stdout 协议调用其 `verify` 命令做 hash 校验。
//! 这里只封装 hash verify，其他 agent 命令（file-tree / pack / unpack 等）
//! 不在本模块重复实现。
//!
//! 协议与 Go 侧一致：
//! - stdin：单个 JSON 请求 `{"proto":1,"cmd":"verify","args":{...}}`
//! - stdout：NDJSON 帧（progress 行 + 一个 result 行）
//! - 退出码：0 成功、1 已处理命令失败、2 协议/传输错误

use crate::error::AnsibleError;
use crate::ssh::SshClient;
use crate::types::{FileCopyOptions, HostConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// 发送给 agent `verify` 命令的清单文件项（路径相对 root）。
#[derive(Debug, Clone, Serialize)]
pub struct AgentVerifyEntry {
    pub path: String,
    pub hash: String,
    pub is_symlink: bool,
}

/// agent `verify` 命令报告的一处 hash/类型不匹配。
#[derive(Debug, Clone, Deserialize)]
pub struct AgentMismatch {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

/// agent `verify` 命令的结果载荷。
#[derive(Debug, Clone, Deserialize)]
pub struct AgentVerifyData {
    pub ok: bool,
    pub file_count: i64,
    pub mismatches: Vec<AgentMismatch>,
}

/// 与 Go agent 协议版本一致。
const AGENT_PROTO: i64 = 1;
/// 与 Go `agent/protocol.AgentVersion` 一致（仅用于日志/排查）。
const AGENT_VERSION: &str = "0.1.2";

/// 最小 agent 客户端：连接一个目标主机，负责上传 agent 并调用 `verify`。
pub struct AgentClient {
    client: SshClient,
    /// 已解析的 agent 二进制（同一连接只解析/上传一次）。
    binary: Option<Vec<u8>>,
    /// 已安装的远程路径（同一连接只上传一次）。
    remote_path: Option<String>,
}

impl AgentClient {
    /// 通过 SSH 连接目标主机。
    pub fn connect(config: HostConfig) -> Result<Self, AnsibleError> {
        Ok(Self {
            client: SshClient::new(config)?,
            binary: None,
            remote_path: None,
        })
    }

    /// 调用 agent `verify` 校验 `root` 下的文件是否与清单一致。
    ///
    /// `files` 的 `path` 是相对 `root` 的路径（与 DB/manifest 中的
    /// `file_path` 一致）。返回 agent 侧比对结果；清单中不存在的文件会以
    /// `(missing)` 形式进入 mismatches，而不是整个命令失败。
    pub fn verify_files(
        &mut self,
        root: &str,
        files: &[AgentVerifyEntry],
    ) -> Result<AgentVerifyData, AnsibleError> {
        let remote_path = self.ensure_installed()?;

        let manifest = serde_json::json!({ "files": files });
        let request = serde_json::json!({
            "proto": AGENT_PROTO,
            "cmd": "verify",
            "args": {
                "root": root,
                "manifest": manifest,
            },
        });
        let request_json = serde_json::to_vec(&request).map_err(|e| {
            AnsibleError::AgentError(format!("failed to encode verify request: {}", e))
        })?;

        info!(
            "Agent verify: {} files under '{}' on host '{}' (agent: {})",
            files.len(),
            root,
            self.client.get_host_config().hostname,
            remote_path
        );

        let result = self
            .client
            .execute_command_with_input(&remote_path, &request_json)?;
        parse_verify_result(&result.stdout, &result.stderr, result.exit_code)
    }

    /// 确保 agent 二进制已在目标主机上就绪（探测架构 -> 解析二进制 ->
    /// 幂等上传），返回远程路径。
    fn ensure_installed(&mut self) -> Result<String, AnsibleError> {
        if let Some(path) = &self.remote_path {
            return Ok(path.clone());
        }

        let arch = self.detect_remote_arch()?;
        let binary = match &self.binary {
            Some(b) => b.clone(),
            None => resolve_agent_binary(&arch)?,
        };

        let digest = Sha256::digest(&binary);
        let digest_hex = format!("{:x}", digest);
        let remote_path = format!("/tmp/adms-agent-{}", &digest_hex[..12]);

        // 写入临时本地文件，复用 SshClient 的带 hash 校验传输（目标已存在
        // 且 hash 一致时自动跳过，保证幂等）。
        let temp_path = std::env::temp_dir().join(format!("adms-agent-{}.bin", &digest_hex[..12]));
        let write_result = fs::write(&temp_path, &binary);
        if let Err(e) = write_result {
            return Err(AnsibleError::AgentError(format!(
                "failed to stage agent binary at {}: {}",
                temp_path.display(),
                e
            )));
        }

        let options = FileCopyOptions {
            mode: Some("0755".to_string()),
            backup: false,
            create_dirs: false,
            precomputed_hash: Some(digest_hex),
            ..Default::default()
        };
        let transfer = self
            .client
            .copy_file_to_remote_with_options(&temp_path.to_string_lossy(), &remote_path, &options);

        let _ = fs::remove_file(&temp_path);
        transfer.map_err(|e| {
            AnsibleError::AgentError(format!(
                "failed to upload agent binary to '{}': {}",
                remote_path, e
            ))
        })?;

        info!(
            "Go agent v{} ready on host '{}' at {} (linux/{}, {} bytes)",
            AGENT_VERSION,
            self.client.get_host_config().hostname,
            remote_path,
            arch,
            binary.len()
        );

        self.binary = Some(binary);
        self.remote_path = Some(remote_path.clone());
        Ok(remote_path.to_string())
    }

    /// 探测远程架构（`uname -m`），与 Go 侧 `detectRemoteArch` 一致。
    fn detect_remote_arch(&self) -> Result<String, AnsibleError> {
        let result = self.client.execute_command("uname -m")?;
        if result.exit_code != 0 {
            return Err(AnsibleError::AgentError(format!(
                "failed to detect remote arch on '{}': {}",
                self.client.get_host_config().hostname,
                result.stderr
            )));
        }
        let arch = map_uname_arch(&result.stdout).ok_or_else(|| {
            AnsibleError::AgentError(format!(
                "unsupported remote architecture on '{}': {}",
                self.client.get_host_config().hostname,
                result.stdout.trim()
            ))
        })?;
        Ok(arch)
    }
}

/// 把 `uname -m` 输出归一化为 agent 构建目标架构名。
fn map_uname_arch(output: &str) -> Option<String> {
    let arch = output.trim().to_ascii_lowercase();
    match arch.as_str() {
        "x86_64" | "amd64" => Some("amd64".to_string()),
        "aarch64" | "arm64" => Some("arm64".to_string()),
        _ => None,
    }
}

/// 解析指定架构的 agent 二进制。
///
/// 解析顺序（与 Go 侧 `BinaryForTarget` 一致）：
/// 1. `ADMS_AGENT_BIN_LINUX_<ARCH>`（如 `ADMS_AGENT_BIN_LINUX_AMD64`）
/// 2. `ADMS_AGENT_BIN`
/// 3. 仓库默认路径 `wails-adms-installer/backend/agent/agentbin/`
fn resolve_agent_binary(arch: &str) -> Result<Vec<u8>, AnsibleError> {
    let arch_upper = arch.to_ascii_uppercase();
    let per_arch_env = format!("ADMS_AGENT_BIN_LINUX_{}", arch_upper);
    for env_name in [per_arch_env.as_str(), "ADMS_AGENT_BIN"] {
        if let Ok(path) = std::env::var(env_name) {
            if !path.is_empty() {
                return fs::read(&path).map_err(|e| {
                    AnsibleError::AgentError(format!(
                        "failed to read agent binary from {} ({}): {}",
                        env_name, path, e
                    ))
                });
            }
        }
    }

    for dir in default_agent_bin_dirs() {
        let path = dir.join(format!("adms-agent-linux-{}", arch));
        if path.is_file() {
            return fs::read(&path).map_err(|e| {
                AnsibleError::AgentError(format!(
                    "failed to read agent binary at {}: {}",
                    path.display(),
                    e
                ))
            });
        }
    }

    Err(AnsibleError::AgentError(format!(
        "no Go agent binary for linux/{}: set ADMS_AGENT_BIN_LINUX_{} or ADMS_AGENT_BIN, \
         or build it with wails-adms-installer/backend/agent/build.sh",
        arch, arch_upper
    )))
}

/// 候选的 agent 二进制目录。
fn default_agent_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 以 rs-ansible 所在仓库为锚点：<workspace>/wails-adms-installer/backend/agent/agentbin
    if let Some(crate_dir) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() {
        if let Some(workspace_root) = crate_dir.parent() {
            dirs.push(
                workspace_root
                    .join("wails-adms-installer")
                    .join("backend")
                    .join("agent")
                    .join("agentbin"),
            );
        }
    }

    // 开发期从仓库根目录运行时的兜底。
    dirs.push(PathBuf::from("wails-adms-installer/backend/agent/agentbin"));
    dirs
}

/// 解析 agent `verify` 命令的 NDJSON 输出，返回比对结果。
fn parse_verify_result(
    stdout: &str,
    stderr: &str,
    exit_code: i32,
) -> Result<AgentVerifyData, AnsibleError> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let frame: AgentFrame = serde_json::from_str(line).map_err(|e| {
            AnsibleError::AgentError(format!("invalid agent NDJSON frame: {}", e))
        })?;

        match frame.frame_type.as_str() {
            // 进度帧只用于长任务，hash verify 不需要消费。
            "progress" => {}
            "result" => {
                let result = frame.result.ok_or_else(|| {
                    AnsibleError::AgentError("agent result frame without payload".to_string())
                })?;
                if !result.ok && !result.error.is_empty() {
                    return Err(AnsibleError::AgentError(format!(
                        "agent verify command failed: {}",
                        result.error
                    )));
                }
                let data = result.data.ok_or_else(|| {
                    AnsibleError::AgentError("agent verify returned no data".to_string())
                })?;
                return serde_json::from_value(data).map_err(|e| {
                    AnsibleError::AgentError(format!(
                        "failed to decode agent verify data: {}",
                        e
                    ))
                });
            }
            other => {
                return Err(AnsibleError::AgentError(format!(
                    "unknown agent frame type: {}",
                    other
                )));
            }
        }
    }

    if exit_code != 0 {
        return Err(AnsibleError::AgentError(format!(
            "agent exited with {}: {}",
            exit_code,
            stderr.trim()
        )));
    }
    Err(AnsibleError::AgentError(
        "agent produced no result frame".to_string(),
    ))
}

/// NDJSON 帧（仅解析 result 所需字段；progress 帧的字段不需要）。
#[derive(Debug, Deserialize)]
struct AgentFrame {
    #[serde(rename = "type")]
    frame_type: String,
    result: Option<AgentResultFrame>,
}

#[derive(Debug, Deserialize)]
struct AgentResultFrame {
    ok: bool,
    #[serde(default)]
    error: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_arch_normalizes_common_outputs() {
        assert_eq!(map_uname_arch("x86_64\n"), Some("amd64".to_string()));
        assert_eq!(map_uname_arch("amd64"), Some("amd64".to_string()));
        assert_eq!(map_uname_arch("aarch64\n"), Some("arm64".to_string()));
        assert_eq!(map_uname_arch("arm64"), Some("arm64".to_string()));
        assert_eq!(map_uname_arch("i386"), None);
    }

    #[test]
    fn parse_verify_result_ignores_progress_and_returns_data() {
        let stdout = concat!(
            "{\"type\":\"progress\",\"progress\":{\"current\":1,\"total\":2,\"message\":\"50%\"}}\n",
            "{\"type\":\"result\",\"result\":{\"ok\":true,\"data\":{\"ok\":false,\"file_count\":2,",
            "\"mismatches\":[{\"path\":\"bin/app\",\"expected\":\"abc\",\"actual\":\"def\"},",
            "{\"path\":\"conf/missing\",\"expected\":\"aaa\",\"actual\":\"(missing)\"}]}}}\n"
        );
        let data = parse_verify_result(stdout, "", 0).unwrap();
        assert!(!data.ok);
        assert_eq!(data.file_count, 2);
        assert_eq!(data.mismatches.len(), 2);
        assert_eq!(data.mismatches[0].actual, "def");
        assert_eq!(data.mismatches[1].actual, "(missing)");
    }

    #[test]
    fn parse_verify_result_returns_command_error() {
        let stdout =
            "{\"type\":\"result\",\"result\":{\"ok\":false,\"error\":\"verify: root is required\"}}\n";
        let err = parse_verify_result(stdout, "", 1).unwrap_err();
        assert!(err.to_string().contains("root is required"));
    }

    #[test]
    fn parse_verify_result_reports_missing_result_frame() {
        let stdout = "{\"type\":\"progress\",\"progress\":{\"current\":1,\"total\":1}}\n";
        let err = parse_verify_result(stdout, "", 0).unwrap_err();
        assert!(err.to_string().contains("no result frame"));
    }
}
