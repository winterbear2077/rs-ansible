use crate::error::AnsibleError;
use crate::ssh::SshClient;
use crate::types::{CommandResult, FileCopyOptions, FileTransferResult, HostConfig, SystemInfo};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
use tracing::info;
#[derive(Default)]
pub struct AnsibleManager {
    hosts: HashMap<String, HostConfig>,
    max_concurrent_connections: usize,
}

#[derive(Debug, Serialize, Default)]
pub struct BatchResult<T> {
    pub results: HashMap<String, Result<T, AnsibleError>>,
    pub successful: Vec<String>,
    pub failed: Vec<String>,
}

impl<T> BatchResult<T> {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            successful: Vec::new(),
            failed: Vec::new(),
        }
    }

    pub fn add_result(&mut self, host: String, result: Result<T, AnsibleError>) {
        match result {
            Ok(_) => self.successful.push(host.clone()),
            Err(_) => self.failed.push(host.clone()),
        }
        self.results.insert(host, result);
    }

    pub fn success_rate(&self) -> f32 {
        if self.results.is_empty() {
            return 0.0;
        }
        self.successful.len() as f32 / self.results.len() as f32
    }
}

impl AnsibleManager {
    pub fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            max_concurrent_connections: 15, // 默认最大10个并发连接
        }
    }

    /// 设置最大并发连接数
    pub fn with_max_concurrent_connections(mut self, max_connections: usize) -> Self {
        self.max_concurrent_connections = max_connections;
        self
    }

    /// 设置最大并发连接数（可变引用）
    pub fn set_max_concurrent_connections(&mut self, max_connections: usize) {
        self.max_concurrent_connections = max_connections;
    }

    /// 获取当前并发限制
    pub fn get_max_concurrent_connections(&self) -> usize {
        self.max_concurrent_connections
    }

    pub fn add_host(&mut self, name: String, config: HostConfig) {
        self.hosts.insert(name, config);
    }

    pub fn remove_host(&mut self, name: &str) -> Option<HostConfig> {
        self.hosts.remove(name)
    }

    pub fn get_host(&self, name: &str) -> Option<&HostConfig> {
        self.hosts.get(name)
    }

    pub fn list_hosts(&self) -> Vec<&String> {
        self.hosts.keys().collect()
    }

    /// 对所有主机执行ping操作
    pub async fn ping_all(&self) -> BatchResult<bool> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.ping_hosts(&host_names).await
    }

    /// 对指定主机列表执行ping操作（带并发控制）
    pub async fn ping_hosts(&self, host_names: &[String]) -> BatchResult<bool> {
        self.execute_concurrent_operation(host_names, |client| async move { client.ping() })
            .await
    }

    /// 对所有主机执行命令
    pub async fn execute_command_all(&self, command: &str) -> BatchResult<CommandResult> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.execute_command_on_hosts(command, &host_names).await
    }

    /// 对指定主机列表执行命令（带并发控制）
    pub async fn execute_command_on_hosts(
        &self,
        command: &str,
        host_names: &[String],
    ) -> BatchResult<CommandResult> {
        let command = command.to_string();
        self.execute_concurrent_operation(host_names, move |client| {
            let cmd = command.clone();
            async move { client.execute_command(&cmd) }
        })
        .await
    }

    /// 向所有主机复制文件
    pub async fn copy_file_to_all(
        &self,
        local_path: &str,
        remote_path: &str,
    ) -> BatchResult<FileTransferResult> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.copy_file_to_hosts(local_path, remote_path, &host_names)
            .await
    }

    /// 向所有主机复制文件（带选项）
    pub async fn copy_file_to_all_with_options(
        &self,
        local_path: &str,
        remote_path: &str,
        options: &FileCopyOptions,
    ) -> BatchResult<FileTransferResult> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.copy_file_to_hosts_with_options(local_path, remote_path, &host_names, options)
            .await
    }

    /// 向指定主机列表复制文件（带并发控制）
    pub async fn copy_file_to_hosts(
        &self,
        local_path: &str,
        remote_path: &str,
        host_names: &[String],
    ) -> BatchResult<FileTransferResult> {
        self.copy_file_to_hosts_with_options(
            local_path,
            remote_path,
            host_names,
            &FileCopyOptions::default(),
        )
        .await
    }

    /// 向指定主机列表复制文件（带选项和并发控制）
    pub async fn copy_file_to_hosts_with_options(
        &self,
        local_path: &str,
        remote_path: &str,
        host_names: &[String],
        options: &FileCopyOptions,
    ) -> BatchResult<FileTransferResult> {
        let local_path = local_path.to_string();
        let remote_path = remote_path.to_string();
        let options = options.clone();
        self.execute_concurrent_operation(host_names, move |client| {
            let local = local_path.clone();
            let remote = remote_path.clone();
            let opts = options.clone();
            async move { client.copy_file_to_remote_with_options(&local, &remote, &opts) }
        })
        .await
    }

    /// 获取所有主机的系统信息
    pub async fn get_system_info_all(&self) -> BatchResult<SystemInfo> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.get_system_info_from_hosts(&host_names).await
    }

    /// 获取指定主机列表的系统信息（带并发控制）
    pub async fn get_system_info_from_hosts(
        &self,
        host_names: &[String],
    ) -> BatchResult<SystemInfo> {
        self.execute_concurrent_operation(
            host_names,
            |client| async move { client.get_system_info() },
        )
        .await
    }

    /// 在所有主机上管理用户
    pub async fn manage_user_all(
        &self,
        options: &crate::types::UserOptions,
    ) -> BatchResult<crate::types::UserResult> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.manage_user_on_hosts(options, &host_names).await
    }

    /// 在指定主机列表上管理用户（带并发控制）
    pub async fn manage_user_on_hosts(
        &self,
        options: &crate::types::UserOptions,
        host_names: &[String],
    ) -> BatchResult<crate::types::UserResult> {
        let options = options.clone();
        self.execute_concurrent_operation(host_names, move |client| {
            let opts = options.clone();
            async move { client.manage_user(&opts) }
        })
        .await
    }

    /// 向所有主机部署模板
    pub async fn deploy_template_to_all(
        &self,
        options: &crate::types::TemplateOptions,
    ) -> BatchResult<crate::types::TemplateResult> {
        let host_names: Vec<String> = self.hosts.keys().cloned().collect();
        self.deploy_template_to_hosts(options, &host_names).await
    }

    /// 向指定主机列表部署模板（带并发控制）
    pub async fn deploy_template_to_hosts(
        &self,
        options: &crate::types::TemplateOptions,
        host_names: &[String],
    ) -> BatchResult<crate::types::TemplateResult> {
        let options = options.clone();
        self.execute_concurrent_operation(host_names, move |client| {
            let opts = options.clone();
            async move { client.deploy_template(&opts) }
        })
        .await
    }

    /// 通用的并发操作执行器
    pub async fn execute_concurrent_operation<T, F, Fut>(
        &self,
        host_names: &[String],
        operation: F,
    ) -> BatchResult<T>
    where
        T: Send + 'static,
        F: Fn(SshClient) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<T, AnsibleError>> + Send + 'static,
    {
        let mut result = BatchResult::new();

        // 创建信号量来控制并发数
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_connections));
        let mut handles = Vec::new();

        info!(
            "Starting concurrent operation on {} hosts with max {} concurrent connections",
            host_names.len(),
            self.max_concurrent_connections
        );

        for host_name in host_names {
            if let Some(config) = self.hosts.get(host_name) {
                let config = config.clone();
                let host_name = host_name.clone();
                let semaphore = semaphore.clone();
                let operation = operation.clone();

                let handle = task::spawn(async move {
                    // 测试日志：确认日志是否能正确输出
                    tracing::info!("Task started for host: {}", host_name);

                    // 获取信号量许可（限制并发数）
                    let _permit = semaphore.acquire().await.expect("Semaphore closed");

                    tracing::info!("Semaphore acquired for host: {}", host_name);

                    let client_result = SshClient::new(config);
                    match client_result {
                        Ok(client) => {
                            tracing::info!("SSH client created for host: {}", host_name);
                            let op_result = operation(client).await;
                            (host_name, op_result)
                        }
                        Err(e) => (host_name, Err(e)),
                    }
                });
                handles.push(handle);
            } else {
                result.add_result(
                    host_name.clone(),
                    Err(AnsibleError::SshConnectionError(format!(
                        "Host {} not found",
                        host_name
                    ))),
                );
            }
        }

        // 等待所有任务完成
        for handle in handles {
            if let Ok((host_name, op_result)) = handle.await {
                result.add_result(host_name, op_result);
            }
        }

        info!(
            "Concurrent operation completed. Success rate: {:.2}%",
            result.success_rate() * 100.0
        );
        result
    }

    /// 批量操作统计信息
    pub async fn get_batch_operation_stats(&self, host_names: &[String]) -> BatchOperationStats {
        BatchOperationStats {
            total_hosts: host_names.len(),
            max_concurrent: self.max_concurrent_connections,
            estimated_duration_seconds: self.estimate_operation_duration(host_names.len()),
        }
    }

    /// 估算操作持续时间
    fn estimate_operation_duration(&self, host_count: usize) -> f32 {
        let batches = (host_count as f32 / self.max_concurrent_connections as f32).ceil();
        let avg_operation_time = 5.0; // 假设每个操作平均需要5秒
        batches * avg_operation_time
    }

    /// 创建主机配置构建器
    pub fn host_builder() -> HostConfigBuilder {
        HostConfigBuilder::new()
    }
}

#[derive(Debug, Serialize)]
pub struct BatchOperationStats {
    pub total_hosts: usize,
    pub max_concurrent: usize,
    pub estimated_duration_seconds: f32,
}

#[derive(Default)]
pub struct HostConfigBuilder {
    config: HostConfig,
}

impl HostConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: HostConfig::default(),
        }
    }

    pub fn hostname(mut self, hostname: &str) -> Self {
        self.config.hostname = hostname.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn username(mut self, username: &str) -> Self {
        self.config.username = username.to_string();
        self
    }

    pub fn password(mut self, password: &str) -> Self {
        self.config.password = Some(password.to_string());
        self
    }

    pub fn private_key_path(mut self, path: &str) -> Self {
        self.config.private_key_path = Some(path.to_string());
        self
    }

    pub fn passphrase(mut self, passphrase: &str) -> Self {
        self.config.passphrase = Some(passphrase.to_string());
        self
    }

    pub fn build(self) -> HostConfig {
        self.config
    }
}
