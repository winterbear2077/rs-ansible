use crate::error::AnsibleError;
use crate::ssh::client::SshClient;
use crate::types::FileHashInfo;

impl SshClient {
    /// 计算本地文件的 hash 值
    pub(super) fn calculate_local_file_hash(
        &self,
        local_path: &str,
        algorithm: &str,
    ) -> Result<FileHashInfo, AnsibleError> {
        let hash = crate::utils::calculate_file_hash(local_path, algorithm)?;
        let metadata = std::fs::metadata(local_path).map_err(|e| {
            AnsibleError::FileOperationError(format!("Failed to get file metadata: {}", e))
        })?;

        Ok(FileHashInfo {
            algorithm: algorithm.to_string(),
            hash,
            size: metadata.len(),
        })
    }

    /// 获取远程文件的 hash 值
    pub(super) fn get_remote_file_hash(
        &self,
        remote_path: &str,
        algorithm: &str,
    ) -> Result<Option<FileHashInfo>, AnsibleError> {
        // 首先检查文件是否存在
        let check_cmd = format!(
            "test -f '{}' && echo 'exists' || echo 'not_exists'",
            remote_path
        );
        let check_result = self.execute_command(&check_cmd)?;

        if check_result.stdout.trim() == "not_exists" {
            return Ok(None);
        }

        // 获取文件大小
        let size_cmd = format!(
            "stat -c %s '{}' 2>/dev/null || stat -f %z '{}'",
            remote_path, remote_path
        );
        let size_result = self.execute_command(&size_cmd)?;
        let size: u64 = size_result.stdout.trim().parse().map_err(|e| {
            AnsibleError::FileOperationError(format!("Failed to parse file size: {}", e))
        })?;

        // 计算远程文件 hash
        let hash_cmd = match algorithm.to_lowercase().as_str() {
            "sha256" => format!(
                "sha256sum '{}' 2>/dev/null || shasum -a 256 '{}'",
                remote_path, remote_path
            ),
            "md5" => format!(
                "md5sum '{}' 2>/dev/null || md5 -r '{}'",
                remote_path, remote_path
            ),
            _ => {
                return Err(AnsibleError::FileOperationError(format!(
                    "Unsupported hash algorithm: {}",
                    algorithm
                )));
            }
        };

        let hash_result = self.execute_command(&hash_cmd)?;

        if hash_result.exit_code != 0 {
            return Err(AnsibleError::FileOperationError(format!(
                "Failed to calculate remote file hash: {}",
                hash_result.stderr
            )));
        }

        // 解析 hash 输出（不同系统格式可能不同）
        let hash = hash_result
            .stdout
            .split_whitespace()
            .next()
            .ok_or_else(|| {
                AnsibleError::FileOperationError("Failed to parse hash output".to_string())
            })?
            .to_string();

        Ok(Some(FileHashInfo {
            algorithm: algorithm.to_string(),
            hash,
            size,
        }))
    }
}
