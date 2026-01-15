use crate::error::AnsibleError;
use crate::types::{CommandResult, HostConfig};
use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::Path;
use tracing::info;

/// SSH 客户端
pub struct SshClient {
    pub(super) session: Session,
    #[allow(dead_code)]
    pub(super) config: HostConfig,
}

impl SshClient {
    /// 创建新的 SSH 连接
    pub fn new(config: HostConfig) -> Result<Self, AnsibleError> {
        let tcp =
            TcpStream::connect(format!("{}:{}", config.hostname, config.port)).map_err(|e| {
                AnsibleError::SshConnectionError(format!(
                    "Failed to connect to {}:{}: {}",
                    config.hostname, config.port, e
                ))
            })?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

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

        Ok(Self { session, config })
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
            "Command '{}' executed with exit code: {}",
            command, exit_code
        );

        Ok(CommandResult {
            exit_code,
            stdout,
            stderr,
        })
    }
}
