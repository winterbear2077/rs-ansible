use crate::error::AnsibleError;
use crate::types::{CommandResult, HostConfig};
use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

/// SSH 客户端
pub struct SshClient {
    pub(super) session: Session,
    #[allow(dead_code)]
    pub(super) config: HostConfig,
}

impl SshClient {
    /// 创建新的 SSH 连接（带重试机制）
    pub fn new(config: HostConfig) -> Result<Self, AnsibleError> {
        let max_retries = 3;
        let retry_delay = Duration::from_millis(1000);
        let mut last_error = None;

        for attempt in 1..=max_retries {
            if attempt > 1 {
                info!(
                    "Retrying SSH connection to {}:{} (Attempt {}/{})",
                    config.hostname, config.port, attempt, max_retries
                );
                thread::sleep(retry_delay * (attempt as u32 - 1));
            }

            match Self::connect_once(&config) {
                Ok(client) => return Ok(client),
                Err(e) => {
                    warn!(
                        "SSH connection failed for {}:{}: {}. ",
                        config.hostname, config.port, e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AnsibleError::SshConnectionError("Failed to connect after retries".to_string())
        }))
    }

    /// 执行单次连接尝试
    fn connect_once(config: &HostConfig) -> Result<Self, AnsibleError> {
        let tcp = TcpStream::connect(format!("{}:{}", config.hostname, config.port)).map_err(
            |e| {
                AnsibleError::SshConnectionError(format!(
                    "Failed to connect to {}:{}: {}",
                    config.hostname, config.port, e
                ))
            },
        )?;

        // 优化：禁用 Nagle 算法，减少小包延迟，有助于握手稳定性
        if let Err(e) = tcp.set_nodelay(true) {
            warn!("Failed to set TCP_NODELAY: {}", e);
        }

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        
        // 优化：设置超时时间（10秒），避免握手长时间卡死
        session.set_timeout(10000);
        
        session.handshake().map_err(|e| {
            AnsibleError::SshConnectionError(format!("SSH Handshake failed: {}", e))
        })?;

        // 认证
        if let Some(ref private_key_path) = config.private_key_path {
            let passphrase = config.passphrase.as_deref();
            session.userauth_pubkey_file(
                &config.username,
                None,
                Path::new(private_key_path),
                passphrase,
            )?;
        } else if let Some(ref password) = config.password {
            session.userauth_password(&config.username, password)?;
        } else {
            return Err(AnsibleError::AuthenticationError(
                "No authentication method provided".to_string(),
            ));
        }

        if !session.authenticated() {
            return Err(AnsibleError::AuthenticationError(
                "Authentication failed".to_string(),
            ));
        }

        info!("Successfully connected to {}", config.hostname);

        Ok(Self {
            session,
            config: config.clone(),
        })
    }

    /// 获取当前主机的配置信息
    pub fn get_host_config(&self) -> &HostConfig {
        &self.config
    }

    /// 测试连接是否正常
    pub fn ping(&self) -> Result<bool, AnsibleError> {
        let result = self.execute_command("echo 'pong'")?;
        Ok(result.exit_code == 0 && result.stdout.trim() == "pong")
    }

    /// 执行远程命令
    pub fn execute_command(&self, command: &str) -> Result<CommandResult, AnsibleError> {
        let mut channel = self.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        channel.read_to_string(&mut stdout)?;
        channel.stderr().read_to_string(&mut stderr)?;

        channel.wait_close()?;
        let exit_code = channel.exit_status()?;

        info!(
            "Command '{}' on '{}' executed with exit code: {}",
            command, self.config.hostname, exit_code
        );

        Ok(CommandResult {
            exit_code,
            stdout,
            stderr,
        })
    }
}
