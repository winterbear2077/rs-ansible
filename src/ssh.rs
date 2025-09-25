use crate::error::AnsibleError;
use crate::types::{HostConfig, SystemInfo, CommandResult, FileTransferResult, NetworkInterface, FileCopyOptions};
use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::Path;
use std::collections::HashMap;
use log::info;

pub struct SshClient {
    session: Session,
    #[allow(dead_code)]
    config: HostConfig,
}

impl SshClient {
    pub fn new(config: HostConfig) -> Result<Self, AnsibleError> {
        let tcp = TcpStream::connect(format!("{}:{}", config.hostname, config.port))
            .map_err(|e| AnsibleError::SshConnectionError(format!("Failed to connect to {}:{}: {}", config.hostname, config.port, e)))?;
        
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        
        // 认证
        if let Some(ref private_key_path) = config.private_key_path {
            let passphrase = config.passphrase.as_deref();
            session.userauth_pubkey_file(&config.username, None, Path::new(private_key_path), passphrase)?;
        } else if let Some(ref password) = config.password {
            session.userauth_password(&config.username, password)?;
        } else {
            return Err(AnsibleError::AuthenticationError("No authentication method provided".to_string()));
        }
        
        if !session.authenticated() {
            return Err(AnsibleError::AuthenticationError("Authentication failed".to_string()));
        }
        
        info!("Successfully connected to {}", config.hostname);
        
        Ok(Self { session, config })
    }
    
    pub fn ping(&self) -> Result<bool, AnsibleError> {
        let result = self.execute_command("echo 'pong'")?;
        Ok(result.exit_code == 0 && result.stdout.trim() == "pong")
    }
    
    pub fn execute_command(&self, command: &str) -> Result<CommandResult, AnsibleError> {
        let mut channel = self.session.channel_session()?;
        channel.exec(command)?;
        
        let mut stdout = String::new();
        let mut stderr = String::new();
        
        channel.read_to_string(&mut stdout)?;
        channel.stderr().read_to_string(&mut stderr)?;
        
        channel.wait_close()?;
        let exit_code = channel.exit_status()?;
        
        info!("Command '{}' executed with exit code: {}", command, exit_code);
        
        Ok(CommandResult {
            exit_code,
            stdout,
            stderr,
        })
    }
    
    pub fn copy_file_to_remote(&self, local_path: &str, remote_path: &str) -> Result<FileTransferResult, AnsibleError> {
        self.copy_file_to_remote_with_options(local_path, remote_path, &FileCopyOptions::default())
    }

    pub fn copy_file_to_remote_with_options(&self, local_path: &str, remote_path: &str, options: &FileCopyOptions) -> Result<FileTransferResult, AnsibleError> {
        let local_file = std::fs::File::open(local_path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to open local file {}: {}", local_path, e)))?;
        
        let metadata = local_file.metadata()
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to get file metadata: {}", e)))?;
        
        let file_size = metadata.len();

        // 创建目录（如果需要）
        if options.create_dirs {
            if let Some(parent_dir) = Path::new(remote_path).parent() {
                let parent_str = parent_dir.to_string_lossy();
                if !parent_str.is_empty() && parent_str != "/" {
                    let mkdir_cmd = format!("mkdir -p '{}'", parent_str);
                    let mkdir_result = self.execute_command(&mkdir_cmd)?;
                    if mkdir_result.exit_code != 0 {
                        return Err(AnsibleError::FileOperationError(
                            format!("Failed to create directory {}: {}", parent_str, mkdir_result.stderr)
                        ));
                    }
                }
            }
        }

        // 备份现有文件（如果需要）
        if options.backup {
            let backup_cmd = format!("[ -f '{}' ] && cp '{}' '{}.bak.$(date +%Y%m%d_%H%M%S)' || true", 
                                   remote_path, remote_path, remote_path);
            let backup_result = self.execute_command(&backup_cmd)?;
            if backup_result.exit_code != 0 {
                info!("Backup command failed (file may not exist): {}", backup_result.stderr);
            }
        }

        // 传输文件
        let initial_mode = if let Some(ref mode) = options.mode {
            u32::from_str_radix(mode, 8).unwrap_or(0o644)
        } else {
            0o644
        };

        let mut remote_file = self.session.scp_send(Path::new(remote_path), initial_mode as i32, file_size, None)?;
        
        let mut local_reader = std::io::BufReader::new(local_file);
        let bytes_transferred = std::io::copy(&mut local_reader, &mut remote_file)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to transfer file: {}", e)))?;
        
        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;

        // 设置文件权限（如果指定）
        if let Some(ref mode) = options.mode {
            let chmod_cmd = format!("chmod {} '{}'", mode, remote_path);
            let chmod_result = self.execute_command(&chmod_cmd)?;
            if chmod_result.exit_code != 0 {
                return Err(AnsibleError::FileOperationError(
                    format!("Failed to set file permissions {}: {}", mode, chmod_result.stderr)
                ));
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
                return Err(AnsibleError::FileOperationError(
                    format!("Failed to set file owner {}: {}", chown_user, chown_result.stderr)
                ));
            }
        } else if let Some(ref group) = options.group {
            // 只设置组
            let chgrp_cmd = format!("chgrp {} '{}'", group, remote_path);
            let chgrp_result = self.execute_command(&chgrp_cmd)?;
            if chgrp_result.exit_code != 0 {
                return Err(AnsibleError::FileOperationError(
                    format!("Failed to set file group {}: {}", group, chgrp_result.stderr)
                ));
            }
        }
        
        let mut message = format!("Successfully transferred {} bytes", bytes_transferred);
        if let Some(ref owner) = options.owner {
            message.push_str(&format!(", owner: {}", owner));
        }
        if let Some(ref group) = options.group {
            message.push_str(&format!(", group: {}", group));
        }
        if let Some(ref mode) = options.mode {
            message.push_str(&format!(", mode: {}", mode));
        }

        info!("File {} copied to remote {} ({})", local_path, remote_path, message);
        
        Ok(FileTransferResult {
            success: true,
            bytes_transferred,
            message,
        })
    }
    
    pub fn copy_file_from_remote(&self, remote_path: &str, local_path: &str) -> Result<FileTransferResult, AnsibleError> {
        let (mut remote_file, _stat) = self.session.scp_recv(Path::new(remote_path))?;
        
        let mut local_file = std::fs::File::create(local_path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to create local file {}: {}", local_path, e)))?;
        
        let bytes_transferred = std::io::copy(&mut remote_file, &mut local_file)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to transfer file: {}", e)))?;
        
        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;
        
        info!("File {} copied from remote {} ({} bytes)", remote_path, local_path, bytes_transferred);
        
        Ok(FileTransferResult {
            success: true,
            bytes_transferred,
            message: format!("Successfully transferred {} bytes", bytes_transferred),
        })
    }
    
    pub fn get_system_info(&self) -> Result<SystemInfo, AnsibleError> {
        let hostname = self.execute_command("hostname")?.stdout.trim().to_string();
        let os = self.execute_command("uname -s")?.stdout.trim().to_string();
        let kernel_version = self.execute_command("uname -r")?.stdout.trim().to_string();
        let architecture = self.execute_command("uname -m")?.stdout.trim().to_string();
        let uptime = self.execute_command("uptime")?.stdout.trim().to_string();
        
        // 获取内存信息
        let memory_info = self.execute_command("free -h | grep Mem")?;
        let memory_parts: Vec<&str> = memory_info.stdout.split_whitespace().collect();
        let memory_total = memory_parts.get(1).unwrap_or(&"Unknown").to_string();
        let memory_free = memory_parts.get(3).unwrap_or(&"Unknown").to_string();
        
        // 获取磁盘使用情况
        let disk_info = self.execute_command("df -h")?;
        let mut disk_usage = HashMap::new();
        for line in disk_info.stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                disk_usage.insert(parts[5].to_string(), parts[4].to_string());
            }
        }
        
        // 获取CPU信息
        let cpu_info = self.execute_command("lscpu | grep 'Model name' | cut -d':' -f2 | xargs")?
            .stdout.trim().to_string();
        
        // 获取网络接口信息
        let network_info = self.execute_command("ip addr show")?;
        let mut network_interfaces = Vec::new();
        
        let mut current_interface = String::new();
        for line in network_info.stdout.lines() {
            if line.starts_with(char::is_numeric) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    current_interface = parts[1].trim().to_string();
                }
            } else if line.contains("inet ") && !current_interface.is_empty() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(ip_part) = parts.get(1) {
                    let ip = ip_part.split('/').next().unwrap_or("").to_string();
                    if !ip.is_empty() && ip != "127.0.0.1" {
                        network_interfaces.push(NetworkInterface {
                            name: current_interface.clone(),
                            ip_address: ip,
                            mac_address: "Unknown".to_string(), // 简化处理
                        });
                    }
                }
            }
        }
        
        info!("System info collected for {}", hostname);
        
        Ok(SystemInfo {
            hostname,
            os,
            kernel_version,
            architecture,
            uptime,
            memory_total,
            memory_free,
            disk_usage,
            cpu_info,
            network_interfaces,
        })
    }
}