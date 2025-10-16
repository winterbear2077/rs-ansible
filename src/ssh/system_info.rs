use crate::error::AnsibleError;
use crate::ssh::client::SshClient;
use crate::types::{NetworkInterface, SystemInfo};
use std::collections::HashMap;
use tracing::info;

impl SshClient {
    /// 获取远程主机的系统信息
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
        let cpu_info = self
            .execute_command("lscpu | grep 'Model name' | cut -d':' -f2 | xargs")?
            .stdout
            .trim()
            .to_string();

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
