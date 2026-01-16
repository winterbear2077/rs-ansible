use crate::error::AnsibleError;
use crate::ssh::client::SshClient;
use crate::types::{FileCopyOptions, FileTransferResult};
use crate::utils::generate_remote_temp_path;
use std::path::Path;
use tracing::info;

impl SshClient {
    /// 复制文件到远程主机（使用默认选项）
    pub fn copy_file_to_remote(
        &self,
        local_path: &str,
        remote_path: &str,
    ) -> Result<FileTransferResult, AnsibleError> {
        self.copy_file_to_remote_with_options(local_path, remote_path, &FileCopyOptions::default())
    }

    /// 复制文件到远程主机（带选项）
    pub fn copy_file_to_remote_with_options(
        &self,
        local_path: &str,
        remote_path: &str,
        options: &FileCopyOptions,
    ) -> Result<FileTransferResult, AnsibleError> {
        // 固定使用 SHA256 算法进行完整性验证
        let hash_algorithm = "sha256";

        // ========== 第一次 Hash：计算本地文件 hash（如果提供了预计算 hash 则跳过） ==========
        let local_hash_info = if let Some(ref hash) = options.precomputed_hash {
            info!("[1/3] Using precomputed local file hash (SHA256)...");
            let metadata = std::fs::metadata(local_path).map_err(|e| {
                AnsibleError::FileOperationError(format!("Failed to get file metadata: {}", e))
            })?;
            crate::types::FileHashInfo {
                algorithm: hash_algorithm.to_string(),
                hash: hash.clone(),
                size: metadata.len(),
            }
        } else {
            info!("[1/3] Calculating local file hash (SHA256)...");
            self.calculate_local_file_hash(local_path, hash_algorithm)?
        };

        info!(
            "Local file hash: {} (size: {} bytes)",
            local_hash_info.hash, local_hash_info.size
        );

        // ========== 第二次 Hash：检查远程文件（幂等性检查，总是执行） ==========
        info!("[2/3] Checking remote file for idempotency...");
        match self.get_remote_file_hash(remote_path, hash_algorithm)? {
            Some(remote_hash_info) => {
                // 比较 hash 和大小
                if remote_hash_info.hash == local_hash_info.hash
                    && remote_hash_info.size == local_hash_info.size
                {
                    info!(
                        "Remote file unchanged (hash: {}), skipping transfer",
                        remote_hash_info.hash
                    );

                    // 仍然需要更新权限和所有者（如果指定）
                    self.apply_file_attributes(remote_path, options)?;

                    return Ok(FileTransferResult {
                        success: true,
                        bytes_transferred: 0,
                        message: format!(
                            "File unchanged (hash: {}), attributes updated",
                            remote_hash_info.hash
                        ),
                    });
                } else {
                    info!(
                        "File changed - Local: {}, Remote: {}, will transfer",
                        local_hash_info.hash, remote_hash_info.hash
                    );
                }
            }
            None => {
                info!("Remote file {} does not exist, will transfer", remote_path);
            }
        }

        // ========== 执行实际的文件传输（带原子性保证） ==========
        let local_file = std::fs::File::open(local_path).map_err(|e| {
            AnsibleError::FileOperationError(format!(
                "Failed to open local file {}: {}",
                local_path, e
            ))
        })?;

        let metadata = local_file.metadata().map_err(|e| {
            AnsibleError::FileOperationError(format!("Failed to get file metadata: {}", e))
        })?;

        let file_size = metadata.len();

        // 创建目录（如果需要）
        if options.create_dirs
            && let Some(parent_dir) = Path::new(remote_path).parent() {
                let parent_str = parent_dir.to_string_lossy();
                if !parent_str.is_empty() && parent_str != "/" {
                    let mkdir_cmd = format!("mkdir -p '{}'", parent_str);
                    let mkdir_result = self.execute_command(&mkdir_cmd)?;
                    if mkdir_result.exit_code != 0 {
                        return Err(AnsibleError::FileOperationError(format!(
                            "Failed to create directory {}: {}",
                            parent_str, mkdir_result.stderr
                        )));
                    }
                }
            }

        // 备份现有文件（如果需要）
        if options.backup {
            // 在 Rust 端生成时间戳，避免 shell 命令中的 $() 被当作字面字符串
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let backup_cmd = format!(
                "[ -f '{}' ] && cp '{}' '{}.bak.{}' || true",
                remote_path, remote_path, remote_path, timestamp
            );
            let backup_result = self.execute_command(&backup_cmd)?;
            if backup_result.exit_code != 0 {
                info!(
                    "Backup command failed (file may not exist): {}",
                    backup_result.stderr
                );
            }
        }

        // 使用临时文件进行原子性传输（使用统一的工具函数生成唯一后缀）
        let temp_remote_path = generate_remote_temp_path(remote_path);

        let initial_mode = if let Some(ref mode) = options.mode {
            u32::from_str_radix(mode, 8).unwrap_or(0o644)
        } else {
            0o644
        };

        info!(
            "Transferring file to temporary location: {}",
            temp_remote_path
        );
        let mut remote_file = self.session.scp_send(
            Path::new(&temp_remote_path),
            initial_mode as i32,
            file_size,
            None,
        )?;

        let mut local_reader = std::io::BufReader::new(local_file);
        let bytes_transferred =
            std::io::copy(&mut local_reader, &mut remote_file).map_err(|e| {
                AnsibleError::FileOperationError(format!("Failed to transfer file: {}", e))
            })?;

        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;

        info!("File transferred: {} bytes", bytes_transferred);

        // ========== 第三次 Hash：验证传输后的文件（总是执行，确保传输完整性） ==========
        info!("[3/3] Verifying file integrity after transfer (SHA256, forced)...");
        match self.get_remote_file_hash(&temp_remote_path, hash_algorithm)? {
            Some(remote_hash_info) => {
                // 验证 hash
                if remote_hash_info.hash != local_hash_info.hash {
                    // Hash 不匹配，删除临时文件并报错
                    let _ = self.execute_command(&format!("rm -f '{}'", temp_remote_path));
                    return Err(AnsibleError::FileOperationError(format!(
                        "File transfer verification FAILED! SHA256 hash mismatch detected.\n\
                         Local hash:  {}\n\
                         Local path: {} \n\
                         Remote hash: {}\n\
                         Remote path: {} \n\
                         File may be corrupted during transfer: {}",
                        local_hash_info.hash,
                        local_path,
                        remote_hash_info.hash,
                        temp_remote_path,
                        local_path
                    )));
                }

                // 验证文件大小
                if remote_hash_info.size != local_hash_info.size {
                    let _ = self.execute_command(&format!("rm -f '{}'", temp_remote_path));
                    return Err(AnsibleError::FileOperationError(format!(
                        "File transfer verification FAILED! Size mismatch detected.\n\
                         Local size:  {} bytes\n\
                         Remote size: {} bytes\n\
                         File may be corrupted during transfer: {}",
                        local_hash_info.size,
                        remote_hash_info.size,
                        local_path
                    )));
                }

                info!(
                    "✓ Transfer verification passed! Hash: {} (size: {} bytes)",
                    remote_hash_info.hash, remote_hash_info.size
                );
            }
            None => {
                let _ = self.execute_command(&format!("rm -f '{}'", temp_remote_path));
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to calculate remote file hash after transfer: {}",
                    temp_remote_path
                )));
            }
        }

        // 原子性地移动临时文件到目标位置
        info!("Moving verified file to final destination: {}", remote_path);
        let mv_cmd = format!("mv '{}' '{}'", temp_remote_path, remote_path);
        let mv_result = self.execute_command(&mv_cmd)?;
        if mv_result.exit_code != 0 {
            // 移动失败，清理临时文件
            let _ = self.execute_command(&format!("rm -f '{}'", temp_remote_path));
            return Err(AnsibleError::FileOperationError(format!(
                "Failed to move temp file to destination: {}",
                mv_result.stderr
            )));
        }

        // 应用文件属性（权限、所有者、组）
        self.apply_file_attributes(remote_path, options)?;

        // 构建成功消息
        let mut message = format!(
            "Successfully transferred {} bytes (hash: {})",
            bytes_transferred, local_hash_info.hash
        );
        if let Some(ref owner) = options.owner {
            message.push_str(&format!(", owner: {}", owner));
        }
        if let Some(ref group) = options.group {
            message.push_str(&format!(", group: {}", group));
        }
        if let Some(ref mode) = options.mode {
            message.push_str(&format!(", mode: {}", mode));
        }

        info!(
            "File successfully copied and verified: {} -> {}",
            local_path, remote_path
        );

        Ok(FileTransferResult {
            success: true,
            bytes_transferred,
            message,
        })
    }

    /// 从远程主机复制文件到本地
    pub fn copy_file_from_remote(
        &self,
        remote_path: &str,
        local_path: &str,
    ) -> Result<FileTransferResult, AnsibleError> {
        let (mut remote_file, _stat) = self.session.scp_recv(Path::new(remote_path))?;

        let mut local_file = std::fs::File::create(local_path).map_err(|e| {
            AnsibleError::FileOperationError(format!(
                "Failed to create local file {}: {}",
                local_path, e
            ))
        })?;

        let bytes_transferred = std::io::copy(&mut remote_file, &mut local_file).map_err(|e| {
            AnsibleError::FileOperationError(format!("Failed to transfer file: {}", e))
        })?;

        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;

        info!(
            "File {} copied from remote {} ({} bytes)",
            remote_path, local_path, bytes_transferred
        );

        Ok(FileTransferResult {
            success: true,
            bytes_transferred,
            message: format!("Successfully transferred {} bytes", bytes_transferred),
        })
    }

    /// 应用文件属性（权限、所有者等）
    pub(super) fn apply_file_attributes(
        &self,
        remote_path: &str,
        options: &FileCopyOptions,
    ) -> Result<(), AnsibleError> {
        // 设置文件权限（如果指定）
        if let Some(ref mode) = options.mode {
            let chmod_cmd = format!("chmod {} '{}'", mode, remote_path);
            let chmod_result = self.execute_command(&chmod_cmd)?;
            if chmod_result.exit_code != 0 {
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to set file permissions {}: {}",
                    mode, chmod_result.stderr
                )));
            }
        }

        // 设置文件所有者（如果指定）
        if let Some(ref owner) = options.owner {
            let chown_user = if let Some(ref group) = options.group {
                format!("{}:{}", owner, group)
            } else {
                owner.clone()
            };
            let chown_cmd = format!("chown {} '{}'", chown_user, remote_path);
            let chown_result = self.execute_command(&chown_cmd)?;
            if chown_result.exit_code != 0 {
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to set file owner {}: {}",
                    chown_user, chown_result.stderr
                )));
            }
        } else if let Some(ref group) = options.group {
            // 只设置组
            let chgrp_cmd = format!("chgrp {} '{}'", group, remote_path);
            let chgrp_result = self.execute_command(&chgrp_cmd)?;
            if chgrp_result.exit_code != 0 {
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to set file group {}: {}",
                    group, chgrp_result.stderr
                )));
            }
        }

        Ok(())
    }
}
