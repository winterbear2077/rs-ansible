use crate::error::AnsibleError;
use crate::types::HostConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryConfig {
    pub hosts: HashMap<String, HostConfig>,
    pub groups: HashMap<String, Vec<String>>,
}

impl InventoryConfig {
    pub fn new() -> Self {
        Self {
            hosts: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    /// 从YAML文件加载配置
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, AnsibleError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to read config file: {}", e)))?;
        
        serde_yaml::from_str(&content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to parse YAML: {}", e)))
    }

    /// 从JSON文件加载配置
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, AnsibleError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to read config file: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to parse JSON: {}", e)))
    }

    /// 保存配置到YAML文件
    pub fn save_to_yaml<P: AsRef<Path>>(&self, path: P) -> Result<(), AnsibleError> {
        let yaml_content = serde_yaml::to_string(self)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to serialize to YAML: {}", e)))?;
        
        std::fs::write(path, yaml_content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to write file: {}", e)))
    }

    /// 保存配置到JSON文件
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), AnsibleError> {
        let json_content = serde_json::to_string_pretty(self)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to serialize to JSON: {}", e)))?;
        
        std::fs::write(path, json_content)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to write file: {}", e)))
    }

    /// 添加主机到指定组
    pub fn add_host_to_group(&mut self, host_name: String, group_name: String) {
        self.groups.entry(group_name).or_default().push(host_name);
    }

    /// 获取组内所有主机
    pub fn get_hosts_in_group(&self, group_name: &str) -> Vec<String> {
        self.groups.get(group_name).cloned().unwrap_or_default()
    }

    /// 获取所有组名
    pub fn get_groups(&self) -> Vec<&String> {
        self.groups.keys().collect()
    }
}