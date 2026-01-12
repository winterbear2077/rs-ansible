use crate::error::AnsibleError;
use crate::types::{CommandResult, FileTransferResult, SystemInfo, FileCopyOptions, UserOptions, UserResult, TemplateOptions, TemplateResult};
use crate::manager::{AnsibleManager, BatchResult};
use crate::utils::{generate_local_temp_path, generate_remote_temp_path};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "task_type")]
pub enum TaskType {
    #[serde(rename = "command")]
    Command { cmd: String },
    #[serde(rename = "copy")]
    CopyFile { 
        src: String, 
        dest: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<FileCopyOptions>,
    },
    #[serde(rename = "system_info")]
    GetSystemInfo,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "shell")]
    Shell { script: String },
    #[serde(rename = "user")]
    User { 
        #[serde(flatten)]
        options: UserOptions 
    },
    #[serde(rename = "template")]
    Template { 
        #[serde(flatten)]
        options: TemplateOptions 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    #[serde(flatten)]
    pub task_type: TaskType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<String>>, // 如果为None，则在所有主机上执行
    #[serde(default)]
    pub ignore_errors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub name: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize)]
pub enum TaskResult {
    Command(BatchResult<CommandResult>),
    CopyFile(BatchResult<FileTransferResult>),
    SystemInfo(BatchResult<SystemInfo>),
    Ping(BatchResult<bool>),
    User(BatchResult<UserResult>),
    Template(BatchResult<TemplateResult>),
}

impl TaskResult {
    pub fn success_rate(&self) -> f32 {
        match self {
            TaskResult::Command(r) => r.success_rate(),
            TaskResult::CopyFile(r) => r.success_rate(),
            TaskResult::SystemInfo(r) => r.success_rate(),
            TaskResult::Ping(r) => r.success_rate(),
            TaskResult::User(r) => r.success_rate(),
            TaskResult::Template(r) => r.success_rate(),
        }
    }

    pub fn successful_hosts(&self) -> &Vec<String> {
        match self {
            TaskResult::Command(r) => &r.successful,
            TaskResult::CopyFile(r) => &r.successful,
            TaskResult::SystemInfo(r) => &r.successful,
            TaskResult::Ping(r) => &r.successful,
            TaskResult::User(r) => &r.successful,
            TaskResult::Template(r) => &r.successful,
        }
    }

    pub fn failed_hosts(&self) -> &Vec<String> {
        match self {
            TaskResult::Command(r) => &r.failed,
            TaskResult::CopyFile(r) => &r.failed,
            TaskResult::SystemInfo(r) => &r.failed,
            TaskResult::Ping(r) => &r.failed,
            TaskResult::User(r) => &r.failed,
            TaskResult::Template(r) => &r.failed,
        }
    }

    /// 获取所有失败主机的错误信息
    pub fn get_failures(&self) -> Vec<(String, String)> {
        let mut failures = Vec::new();
        
        match self {
            TaskResult::Command(r) => Self::collect_failures(r, &mut failures),
            TaskResult::CopyFile(r) => Self::collect_failures(r, &mut failures),
            TaskResult::SystemInfo(r) => Self::collect_failures(r, &mut failures),
            TaskResult::Ping(r) => Self::collect_failures(r, &mut failures),
            TaskResult::User(r) => Self::collect_failures(r, &mut failures),
            TaskResult::Template(r) => Self::collect_failures(r, &mut failures),
        }
        
        failures
    }

    fn collect_failures<T>(result: &BatchResult<T>, failures: &mut Vec<(String, String)>) {
        for host in &result.failed {
            if let Some(Err(e)) = result.results.get(host) {
                failures.push((host.clone(), e.to_string()));
            }
        }
    }
}

#[derive(Debug)]
pub struct PlaybookResult {
    pub playbook_name: String,
    pub task_results: Vec<(String, TaskResult)>,
    pub overall_success: bool,
    pub failed_hosts: HashSet<String>,  // 记录所有失败的主机
    pub skipped_hosts: HashSet<String>, // 记录被跳过的主机
}

pub struct TaskExecutor<'a> {
    manager: &'a AnsibleManager,
}

impl<'a> TaskExecutor<'a> {
    pub fn new(manager: &'a AnsibleManager) -> Self {
        Self { manager }
    }

    /// 执行单个任务，排除已失败的主机
    pub async fn execute_task(&self, task: &Task, failed_hosts: &HashSet<String>) -> Result<TaskResult, AnsibleError> {
        info!("Executing task: {}", task.name);

        let all_hosts = if let Some(ref specific_hosts) = task.hosts {
            specific_hosts.clone()
        } else {
            self.manager.list_hosts().into_iter().cloned().collect()
        };

        // 过滤掉已失败的主机
        let active_hosts: Vec<String> = all_hosts
            .iter()
            .filter(|h| !failed_hosts.contains(h.as_str()))
            .cloned()
            .collect();

        // 计算被跳过的主机
        let skipped_hosts: Vec<String> = all_hosts
            .iter()
            .filter(|h| failed_hosts.contains(h.as_str()))
            .cloned()
            .collect();

        if !skipped_hosts.is_empty() {
            info!(
                "Skipping task '{}' on {} failed host(s): {}",
                task.name,
                skipped_hosts.len(),
                skipped_hosts.join(", ")
            );
        }

        if active_hosts.is_empty() {
            warn!("No active hosts available for task '{}'", task.name);
            // 返回一个空的结果，表示所有主机都被跳过
            let mut batch_result = BatchResult::new();
            for host in skipped_hosts {
                batch_result.add_result(
                    host,
                    Err(AnsibleError::SshConnectionError("Host skipped due to previous failure".to_string()))
                );
            }
            return Ok(TaskResult::Ping(batch_result));
        }

        let result = match &task.task_type {
            TaskType::Command { cmd } => {
                let batch_result = self.manager.execute_command_on_hosts(cmd, &active_hosts).await;
                TaskResult::Command(batch_result)
            }
            TaskType::CopyFile { src, dest, options } => {
                let batch_result = if let Some(opts) = options {
                    self.manager.copy_file_to_hosts_with_options(src, dest, &active_hosts, opts).await
                } else {
                    self.manager.copy_file_to_hosts(src, dest, &active_hosts).await
                };
                TaskResult::CopyFile(batch_result)
            }
            TaskType::GetSystemInfo => {
                let batch_result = self.manager.get_system_info_from_hosts(&active_hosts).await;
                TaskResult::SystemInfo(batch_result)
            }
            TaskType::Ping => {
                let batch_result = self.manager.ping_hosts(&active_hosts).await;
                TaskResult::Ping(batch_result)
            }
            TaskType::User { options } => {
                let batch_result = self.manager.manage_user_on_hosts(options, &active_hosts).await;
                TaskResult::User(batch_result)
            }
            TaskType::Template { options } => {
                let batch_result = self.manager.deploy_template_to_hosts(options, &active_hosts).await;
                TaskResult::Template(batch_result)
            }
            TaskType::Shell { script } => {
                // 创建临时脚本文件并执行（使用统一的工具函数生成唯一路径）
                let script_path = generate_remote_temp_path("/tmp/rs_ansible_script.sh");
                let temp_file = generate_local_temp_path("rs_ansible_local_script");
                
                // 确保脚本使用 Unix 换行符 (\n)，避免在 Windows 上生成 \r\n 导致执行失败
                let script_unix = script.replace('\r', "");
                
                // 写入本地临时文件
                std::fs::write(&temp_file, script_unix)
                    .map_err(|e| AnsibleError::FileOperationError(format!("Failed to create script file: {}", e)))?;

                // 复制脚本到远程主机
                let copy_result = self.manager.copy_file_to_hosts(&temp_file, &script_path, &active_hosts).await;
                
                // 如果复制成功，执行脚本
                if copy_result.success_rate() > 0.0 {
                    let exec_cmd = format!("chmod +x {} && {}", script_path, script_path);
                    let batch_result = self.manager.execute_command_on_hosts(&exec_cmd, &active_hosts).await;
                    
                    // 清理远程脚本文件
                    let cleanup_cmd = format!("rm -f {}", script_path);
                    let _ = self.manager.execute_command_on_hosts(&cleanup_cmd, &active_hosts).await;
                    
                    TaskResult::Command(batch_result)
                } else {
                    return Err(AnsibleError::FileOperationError(format!("Failed to copy script to remote hosts: Reason: {:?}", copy_result.results)));
                }
            }
        };

        Ok(result)
    }

    /// 执行整个Playbook，支持主机级别的失败追踪
    pub async fn execute_playbook(&self, playbook: &Playbook) -> Result<PlaybookResult, AnsibleError> {
        info!("Starting playbook execution: {}", playbook.name);

        let mut task_results = Vec::new();
        let mut overall_success = true;
        let mut failed_hosts: HashSet<String> = HashSet::new();

        for task in &playbook.tasks {
            match self.execute_task(task, &failed_hosts).await {
                Ok(result) => {
                    let success = result.success_rate() > 0.0;
                    let task_failed_hosts = result.failed_hosts();
                    let task_successful_hosts = result.successful_hosts();
                    
                    // 记录本次任务失败的主机（不包括ignore_errors的任务）
                    if !task.ignore_errors {
                        for host in task_failed_hosts {
                            if !failed_hosts.contains(host) {
                                info!("Host '{}' failed on task '{}', will be skipped in subsequent tasks", 
                                      host, task.name);
                                failed_hosts.insert(host.clone());
                            }
                        }
                    } else if !task_failed_hosts.is_empty() {
                        info!(
                            "Task '{}' failed on {} host(s) but errors are ignored: {}",
                            task.name,
                            task_failed_hosts.len(),
                            task_failed_hosts.join(", ")
                        );
                    }
                    
                    if !success && !task.ignore_errors {
                        overall_success = false;
                    }
                    
                    info!(
                        "Task '{}' completed - Success: {}/{}, Failed: {}/{}, Skipped: {}", 
                        task.name,
                        task_successful_hosts.len(),
                        task_successful_hosts.len() + task_failed_hosts.len(),
                        task_failed_hosts.len(),
                        task_successful_hosts.len() + task_failed_hosts.len(),
                        failed_hosts.len()
                    );
                    
                    task_results.push((task.name.clone(), result));
                    
                    // 如果所有主机都失败了且不忽略错误，停止执行
                    if !success && !task.ignore_errors {
                        info!("All hosts failed on task '{}', stopping playbook execution", task.name);
                        break;
                    }
                }
                Err(e) => {
                    if !task.ignore_errors {
                        return Err(e);
                    }
                    info!("Task '{}' failed but errors are ignored: {}", task.name, e);
                    overall_success = false;
                }
            }
        }

        // 统计最终被跳过的主机
        let skipped_hosts = failed_hosts.clone();

        Ok(PlaybookResult {
            playbook_name: playbook.name.clone(),
            task_results,
            overall_success,
            failed_hosts,
            skipped_hosts,
        })
    }

    /// 从YAML文件加载并执行Playbook
    pub async fn execute_playbook_from_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<PlaybookResult, AnsibleError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to read playbook file: {}", e)))?;
        
        let playbook: Playbook = serde_yaml::from_str(&content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to parse playbook YAML: {}", e)))?;

        self.execute_playbook(&playbook).await
    }
}

impl Task {
    pub fn command(name: &str, cmd: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Command { cmd: cmd.to_string() },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn copy_file(name: &str, src: &str, dest: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::CopyFile { 
                src: src.to_string(), 
                dest: dest.to_string(),
                options: None,
            },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn copy_file_with_options(name: &str, src: &str, dest: &str, options: FileCopyOptions) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::CopyFile { 
                src: src.to_string(), 
                dest: dest.to_string(),
                options: Some(options),
            },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn ping(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Ping,
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn system_info(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::GetSystemInfo,
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn shell_script(name: &str, script: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Shell { script: script.to_string() },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn user(name: &str, options: UserOptions) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::User { options },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn template(name: &str, options: TemplateOptions) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Template { options },
            hosts: None,
            ignore_errors: false,
        }
    }

    pub fn on_hosts(mut self, hosts: Vec<String>) -> Self {
        self.hosts = Some(hosts);
        self
    }

    pub fn ignore_errors(mut self) -> Self {
        self.ignore_errors = true;
        self
    }
}

impl Playbook {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tasks: Vec::new(),
        }
    }

    pub fn add_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), AnsibleError> {
        let yaml_content = serde_yaml::to_string(self)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to serialize playbook: {}", e)))?;
        
        std::fs::write(path, yaml_content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to write playbook file: {}", e)))
    }
}